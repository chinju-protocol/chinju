# HSM (Hardware Security Module) - Reference Design

## 概要

HSMは秘密鍵の生成・保管・署名を、ソフトウェアから隔離された環境で行う。
C4（権限分散型AI管理）のShamir秘密分散に必須。

### 対応トレイト

```rust
pub trait HardwareSecurityModule: Send + Sync {
    fn secure_erase(&self, slot_id: SlotId) -> Result<(), Self::Error>;
    fn sign(&self, slot_id: SlotId, data: &[u8]) -> Result<Signature, Self::Error>;
    fn verify(&self, pk: &PublicKey, data: &[u8], sig: &Signature) -> Result<bool, Self::Error>;
    fn split_secret(&self, secret: &[u8], k: usize, n: usize) -> Result<Vec<Share>, Self::Error>;
    fn combine_shares(&self, shares: &[Share]) -> Result<Vec<u8>, Self::Error>;
    fn generate_keypair(&self, slot_id: SlotId) -> Result<PublicKey, Self::Error>;
    fn is_hardware_backed(&self) -> bool;
}
```

---

## 1. 設計パターン

### Pattern A: USB HSM（中小規模）

**用途**: スタートアップ、中小企業、開発チーム
**レベル**: Level 2-3
**コスト**: $650-$1,500

```
┌─────────────────────────────────────────────────────┐
│  Server                                             │
│  ┌───────────────────────────────────────────────┐  │
│  │  CHINJU Software Layer                        │  │
│  │  (Rust)                                       │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │ PKCS#11 / Native API     │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  libyubihsm / p11-kit                         │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │ USB                      │
└──────────────────────────│──────────────────────────┘
                           │
┌──────────────────────────│──────────────────────────┐
│  YubiHSM 2               │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  FIPS 140-2 Level 3 Certified                 │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Key Storage (128 slots)                │  │  │
│  │  │  Ed25519 / ECDSA / RSA                  │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Crypto Engine                          │  │  │
│  │  │  AES-256-CCM / HMAC-SHA256              │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**BOM**:

| 製品 | ベンダー | 認証 | 単価 |
|------|---------|------|------|
| YubiHSM 2 | Yubico | FIPS 140-2 L3 | $650 |
| 代替: Nitrokey HSM 2 | Nitrokey | CC EAL4+ | $150 |

**Rust実装例**:

```rust
use yubihsm::{Client, Connector, Credentials, object::Type};

pub struct YubiHsmAdapter {
    client: Client,
}

impl YubiHsmAdapter {
    pub fn connect(url: &str, auth_key_id: u16, password: &str) -> Result<Self, HsmError> {
        let connector = Connector::http(&url.parse()?);
        let client = Client::open(
            connector,
            Credentials::from_password(auth_key_id, password.as_bytes()),
            true, // reconnect on error
        )?;
        Ok(Self { client })
    }
}

impl HardwareSecurityModule for YubiHsmAdapter {
    type Error = HsmError;

    fn generate_keypair(&self, slot_id: SlotId) -> Result<PublicKey, Self::Error> {
        use yubihsm::asymmetric::Algorithm;

        let key_id = self.client.generate_asymmetric_key(
            slot_id.0 as u16,
            "chinju-key".into(),
            vec![yubihsm::Capability::SIGN_EDDSA],
            Algorithm::Ed25519,
        )?;

        let pubkey = self.client.get_public_key(key_id)?;
        Ok(PublicKey::from_bytes(pubkey.as_ref()))
    }

    fn sign(&self, slot_id: SlotId, data: &[u8]) -> Result<Signature, Self::Error> {
        let sig = self.client.sign_ed25519(slot_id.0 as u16, data)?;
        Ok(Signature::from_bytes(&sig.as_ref()))
    }

    fn secure_erase(&self, slot_id: SlotId) -> Result<(), Self::Error> {
        self.client.delete_object(slot_id.0 as u16, Type::AsymmetricKey)?;
        Ok(())
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

### Pattern B: Network HSM（大規模）

**用途**: 金融機関、データセンター、エンタープライズ
**レベル**: Level 3-4
**コスト**: $15,000-$100,000

```
┌─────────────────────────────────────────────────────┐
│  Application Servers (Cluster)                      │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐              │
│  │ Server1 │ │ Server2 │ │ Server3 │              │
│  └────┬────┘ └────┬────┘ └────┬────┘              │
│       │           │           │                     │
│       └───────────┼───────────┘                     │
│                   │ TLS 1.3 (mTLS)                  │
└───────────────────│─────────────────────────────────┘
                    │
┌───────────────────│─────────────────────────────────┐
│  HSM Cluster      │                                 │
│  ┌────────────────┴────────────────┐                │
│  │  Load Balancer                  │                │
│  └────────────────┬────────────────┘                │
│       ┌───────────┼───────────┐                     │
│  ┌────┴────┐ ┌────┴────┐ ┌────┴────┐              │
│  │ HSM-1   │ │ HSM-2   │ │ HSM-3   │              │
│  │ (Active)│ │ (Active)│ │(Standby)│              │
│  └─────────┘ └─────────┘ └─────────┘              │
│  Thales Luna / Entrust nShield / Utimaco          │
└─────────────────────────────────────────────────────┘
```

**BOM**:

| 製品 | ベンダー | 認証 | 単価 |
|------|---------|------|------|
| Luna Network HSM 7 | Thales | FIPS 140-2 L3 | $15,000+ |
| nShield Connect XC | Entrust | FIPS 140-2 L3 | $20,000+ |
| CryptoServer Se | Utimaco | FIPS 140-2 L3 | $18,000+ |

**Rust実装例（PKCS#11経由）**:

```rust
use cryptoki::{context::Pkcs11, mechanism::Mechanism, object::Attribute, session::Session};

pub struct Pkcs11HsmAdapter {
    ctx: Pkcs11,
    session: Session,
}

impl Pkcs11HsmAdapter {
    pub fn connect(library_path: &str, slot: u64, pin: &str) -> Result<Self, HsmError> {
        let ctx = Pkcs11::new(library_path)?;
        ctx.initialize(cryptoki::context::CInitializeArgs::OsThreads)?;

        let slot = ctx.get_slots_with_token()?.get(slot as usize).cloned()
            .ok_or(HsmError::SlotNotFound)?;

        let session = ctx.open_rw_session(slot)?;
        session.login(cryptoki::session::UserType::User, Some(pin))?;

        Ok(Self { ctx, session })
    }
}

impl HardwareSecurityModule for Pkcs11HsmAdapter {
    type Error = HsmError;

    fn sign(&self, slot_id: SlotId, data: &[u8]) -> Result<Signature, Self::Error> {
        let key_handle = self.find_key(slot_id)?;
        let sig = self.session.sign(
            &Mechanism::Ecdsa,
            key_handle,
            data,
        )?;
        Ok(Signature::from_bytes(&sig))
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

### Pattern C: Cloud HSM（ハイブリッド）

**用途**: クラウドネイティブ、マルチクラウド
**レベル**: Level 2-3
**コスト**: $1.5-$5/時間

```
┌─────────────────────────────────────────────────────┐
│  AWS                                                │
│  ┌───────────────────────────────────────────────┐  │
│  │  CloudHSM Cluster                             │  │
│  │  (FIPS 140-2 Level 3)                         │  │
│  └───────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────┤
│  Azure                                              │
│  ┌───────────────────────────────────────────────┐  │
│  │  Azure Dedicated HSM                          │  │
│  │  (FIPS 140-2 Level 3)                         │  │
│  └───────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────┤
│  GCP                                                │
│  ┌───────────────────────────────────────────────┐  │
│  │  Cloud HSM                                    │  │
│  │  (FIPS 140-2 Level 3)                         │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
         │
         │ Unified Interface
         ▼
┌─────────────────────────────────────────────────────┐
│  CHINJU Cloud HSM Adapter                           │
│  ┌───────────────────────────────────────────────┐  │
│  │  Provider Abstraction Layer                   │  │
│  │  (AWS SDK / Azure SDK / GCP SDK)             │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Rust実装例（AWS CloudHSM）**:

```rust
use aws_sdk_cloudhsmv2 as cloudhsm;

pub struct AwsCloudHsmAdapter {
    // CloudHSMはPKCS#11経由でアクセス
    pkcs11: Pkcs11HsmAdapter,
}

impl AwsCloudHsmAdapter {
    pub async fn connect(cluster_id: &str, pin: &str) -> Result<Self, HsmError> {
        // CloudHSM clientはPKCS#11ライブラリ経由
        let pkcs11 = Pkcs11HsmAdapter::connect(
            "/opt/cloudhsm/lib/libcloudhsm_pkcs11.so",
            0,
            pin,
        )?;
        Ok(Self { pkcs11 })
    }
}

impl HardwareSecurityModule for AwsCloudHsmAdapter {
    type Error = HsmError;

    fn sign(&self, slot_id: SlotId, data: &[u8]) -> Result<Signature, Self::Error> {
        self.pkcs11.sign(slot_id, data)
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

## 2. Shamir秘密分散の実装

C4（権限分散型AI管理）で使用するShamir秘密分散をHSM内で実行。

```rust
impl HardwareSecurityModule for SecureHsm {
    fn split_secret(
        &self,
        secret: &[u8],
        threshold: usize,  // k
        total_shares: usize, // n
    ) -> Result<Vec<Share>, Self::Error> {
        // 1. HSM内で多項式を生成
        // f(x) = secret + a1*x + a2*x^2 + ... + a(k-1)*x^(k-1)

        // 2. 各シェア x=1,2,...,n に対して f(x) を計算
        // すべてHSM内で実行（秘密はHSMから出ない）

        // 3. シェアを返却（秘密自体は返さない）
        let shares = self.hsm_native_split(secret, threshold, total_shares)?;
        Ok(shares)
    }

    fn combine_shares(&self, shares: &[Share]) -> Result<Vec<u8>, Self::Error> {
        // ラグランジュ補間でf(0) = secretを復元
        // これもHSM内で実行
        let secret = self.hsm_native_combine(shares)?;
        Ok(secret)
    }
}
```

---

## 3. ベンダー多様性

### 必須構成（Level 2+）

| 役割 | プライマリ | フォールバック |
|------|-----------|---------------|
| 署名 | YubiHSM 2 | AWS CloudHSM |
| 鍵生成 | YubiHSM 2 | Azure Managed HSM |

### 推奨構成（Level 3+）

| 役割 | プライマリ | セカンダリ | ターシャリ |
|------|-----------|-----------|-----------|
| 署名 | Thales Luna | Entrust nShield | AWS CloudHSM |
| 鍵生成 | Thales Luna | Utimaco | Azure HSM |

### 切り替え設定

```toml
[hardware.hsm]
primary = "yubihsm"
fallback = ["cloudhsm", "azure_hsm"]

[hardware.hsm.yubihsm]
url = "http://localhost:12345"
auth_key_id = 1
password_env = "YUBIHSM_PASSWORD"

[hardware.hsm.cloudhsm]
cluster_id = "cluster-xxx"
pin_env = "CLOUDHSM_PIN"

[hardware.hsm.azure_hsm]
vault_url = "https://xxx.vault.azure.net"
```

---

## 4. 運用

### 鍵のライフサイクル

```
1. 生成（HSM内）
   ↓
2. バックアップ（暗号化、M-of-N分散）
   ↓
3. 運用（署名、検証）
   ↓
4. ローテーション（定期）
   ↓
5. 失効/消去（安全消去）
```

### 監査ログ

```rust
pub struct HsmAuditLog {
    timestamp: DateTime<Utc>,
    operation: HsmOperation,
    key_id: KeyId,
    result: Result<(), HsmError>,
    operator: OperatorId,
}

pub enum HsmOperation {
    GenerateKey,
    Sign,
    Verify,
    Erase,
    Export,  // バックアップ用（暗号化）
}
```

---

## 参照文書

- `chinju/traits/hardware_abstraction.md` - トレイト定義
- `chinju/hardware/REQUIREMENTS.md` - 要件
- PKCS#11 v2.40 Specification
- Yubico YubiHSM 2 SDK Documentation
- AWS CloudHSM User Guide
