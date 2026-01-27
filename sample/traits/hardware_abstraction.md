# Hardware Abstraction Layer - トレイト定義

## 概要

CHINJUの物理層セキュリティを実現するためのトレイト（インターフェース）定義。
これらのトレイトは、OSSとして提供されるソフトウェア層と、パートナーが提供するハードウェア実装の境界を定義する。

## トレイト一覧

| トレイト | 用途 | 使用コンポーネント |
|---------|------|------------------|
| `ImmutableStorage` | OTP/eFuseへのアクセス | C1, C4 |
| `SecureRandom` | 量子乱数生成 | C2 |
| `SecureExecution` | TEE内実行 | C1, C2, C3 |
| `HardwareSecurityModule` | HSM操作 | C1, C4 |
| `TimeVerification` | GPS時刻検証 | C1, C3 |
| `HardwareControl` | 物理デバイス制御 | C5 |
| `DataDiode` | 一方向通信 | C3 |

---

## 1. ImmutableStorage

OTPメモリやeFuseなど、物理的に書き換え不可能な記憶域へのインターフェース。

```rust
/// 物理的に不変な記憶域へのアクセス
pub trait ImmutableStorage: Send + Sync {
    /// エラー型
    type Error: std::error::Error;

    /// ルート公開鍵を読み取る
    ///
    /// 一度書き込まれたルート鍵は変更不可能。
    /// AIがこの鍵を書き換えることは物理的に不可能。
    fn read_root_key(&self) -> Result<PublicKey, Self::Error>;

    /// 単調増加カウンタの現在値を取得
    ///
    /// ロールバック攻撃を防止するために使用。
    fn read_counter(&self) -> Result<u64, Self::Error>;

    /// カウンタが指定値以上であることを検証
    fn verify_counter(&self, min_value: u64) -> Result<bool, Self::Error>;

    /// カウンタをインクリメント（不可逆）
    ///
    /// 一度インクリメントしたら戻せない。
    fn increment_counter(&self) -> Result<u64, Self::Error>;

    /// 署名付き閾値レコードを読み取る
    fn read_threshold_record(&self) -> Result<SignedThresholdRecord, Self::Error>;

    /// この実装がハードウェアに裏打ちされているか
    fn is_hardware_backed(&self) -> bool;
}

/// 署名付き閾値レコード
pub struct SignedThresholdRecord {
    /// 認定閾値
    pub theta_cert: f64,
    /// 失効閾値
    pub theta_revoke: f64,
    /// バージョン（単調増加）
    pub version: u64,
    /// 署名者の公開鍵
    pub signer: PublicKey,
    /// 署名
    pub signature: Signature,
}
```

### 実装例

```rust
// モック実装（開発・テスト用）
pub struct MockImmutableStorage {
    root_key: PublicKey,
    counter: AtomicU64,
    threshold: SignedThresholdRecord,
}

impl ImmutableStorage for MockImmutableStorage {
    type Error = MockStorageError;

    fn read_root_key(&self) -> Result<PublicKey, Self::Error> {
        Ok(self.root_key.clone())
    }

    fn is_hardware_backed(&self) -> bool {
        false  // モックはハードウェアではない
    }

    // ...
}

// 本番実装（パートナー提供）
// pub struct YubiHsmOtp { ... }
// pub struct IntelSgxSealing { ... }
```

---

## 2. SecureRandom

物理的に予測不可能な乱数を生成する。

```rust
/// 物理乱数源へのアクセス
pub trait SecureRandom: Send + Sync {
    /// エラー型
    type Error: std::error::Error;

    /// 指定バイト数の真正乱数を生成
    ///
    /// QRNGなど物理的エントロピー源から生成される。
    /// 擬似乱数ではなく、AIには予測不可能。
    fn generate(&self, len: usize) -> Result<Vec<u8>, Self::Error>;

    /// エントロピー品質を検証
    ///
    /// NIST SP 800-90B に基づくエントロピー推定。
    fn verify_entropy(&self) -> Result<f64, Self::Error>;

    /// エントロピープールの状態を取得
    fn pool_status(&self) -> Result<EntropyPoolStatus, Self::Error>;

    /// この実装がハードウェアに裏打ちされているか
    fn is_hardware_backed(&self) -> bool;
}

pub struct EntropyPoolStatus {
    /// 利用可能なエントロピービット数
    pub available_bits: usize,
    /// 最後の補充からの経過時間
    pub last_reseed: Duration,
    /// 健全性
    pub health: HealthStatus,
}

pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Failed { reason: String },
}
```

### 実装例

```rust
// モック実装（決定論的、テスト用）
pub struct MockSecureRandom {
    seed: u64,
}

impl SecureRandom for MockSecureRandom {
    type Error = MockRandomError;

    fn generate(&self, len: usize) -> Result<Vec<u8>, Self::Error> {
        // ChaCha20等の暗号学的PRNGを使用
        // 注意: 本番では使用不可
        let mut rng = ChaCha20Rng::seed_from_u64(self.seed);
        let mut buf = vec![0u8; len];
        rng.fill_bytes(&mut buf);
        Ok(buf)
    }

    fn is_hardware_backed(&self) -> bool {
        false
    }

    // ...
}

// 本番実装（パートナー提供）
// pub struct IdQuantiqueQrng { ... }
// pub struct IntelRdrand { ... }
```

---

## 3. SecureExecution

TEE（Trusted Execution Environment）内での隔離実行。

```rust
/// TEE内での隔離実行
pub trait SecureExecution: Send + Sync {
    /// エラー型
    type Error: std::error::Error;

    /// TEE内で関数を実行
    ///
    /// AIシステムから隔離された環境で実行される。
    /// メモリはAIからアクセス不可能。
    fn execute_in_enclave<F, R>(&self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce() -> R + Send,
        R: Send;

    /// リモートアテステーション
    ///
    /// このTEEが正当なハードウェア上で動作していることを証明。
    fn remote_attestation(&self) -> Result<AttestationReport, Self::Error>;

    /// シーリング（永続化）
    ///
    /// データをTEE固有の鍵で暗号化して保存。
    fn seal(&self, data: &[u8]) -> Result<SealedData, Self::Error>;

    /// アンシーリング（復元）
    fn unseal(&self, sealed: &SealedData) -> Result<Vec<u8>, Self::Error>;

    /// この実装がハードウェアに裏打ちされているか
    fn is_hardware_backed(&self) -> bool;
}

pub struct AttestationReport {
    /// プラットフォーム識別子
    pub platform_id: Vec<u8>,
    /// エンクレーブ測定値
    pub mrenclave: Vec<u8>,
    /// 署名者測定値
    pub mrsigner: Vec<u8>,
    /// レポートデータ
    pub report_data: Vec<u8>,
    /// 署名
    pub signature: Vec<u8>,
}

pub struct SealedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub tag: Vec<u8>,
}
```

---

## 4. HardwareSecurityModule

HSM（ハードウェアセキュリティモジュール）操作。

```rust
/// HSM操作
pub trait HardwareSecurityModule: Send + Sync {
    /// エラー型
    type Error: std::error::Error;

    /// 鍵スロットを安全に消去
    ///
    /// 物理的に鍵を破壊。復元不可能。
    fn secure_erase(&self, slot_id: SlotId) -> Result<(), Self::Error>;

    /// 署名を生成
    ///
    /// 秘密鍵はHSM内に留まり、外部には出ない。
    fn sign(&self, slot_id: SlotId, data: &[u8]) -> Result<Signature, Self::Error>;

    /// 署名を検証
    fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool, Self::Error>;

    /// Shamirの秘密分散でシェアを生成
    fn split_secret(
        &self,
        secret: &[u8],
        threshold: usize,
        total_shares: usize,
    ) -> Result<Vec<Share>, Self::Error>;

    /// シェアから秘密を復元
    fn combine_shares(&self, shares: &[Share]) -> Result<Vec<u8>, Self::Error>;

    /// 鍵ペアを生成
    fn generate_keypair(&self, slot_id: SlotId) -> Result<PublicKey, Self::Error>;

    /// この実装がハードウェアに裏打ちされているか
    fn is_hardware_backed(&self) -> bool;
}

pub struct SlotId(pub u32);

pub struct Share {
    pub index: u8,
    pub data: Vec<u8>,
}
```

---

## 5. TimeVerification

GPS同期による高精度時刻検証。

```rust
/// 時刻検証（光速制約）
pub trait TimeVerification: Send + Sync {
    /// エラー型
    type Error: std::error::Error;

    /// 現在時刻を取得（GPS同期済み）
    fn now(&self) -> Result<Timestamp, Self::Error>;

    /// 時刻同期の精度を取得
    fn sync_precision(&self) -> Result<Duration, Self::Error>;

    /// 地点間の最小物理遅延を取得
    ///
    /// 光速と伝送媒体の屈折率に基づく理論下限値。
    fn min_delay(&self, from: Location, to: Location) -> Duration;

    /// 因果律を検証
    ///
    /// 複数イベントのタイムスタンプが物理的に可能かを検証。
    fn verify_causality(
        &self,
        events: &[(Location, Timestamp)],
    ) -> Result<bool, Self::Error>;

    /// この実装がハードウェアに裏打ちされているか
    fn is_hardware_backed(&self) -> bool;
}

pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

pub struct Timestamp {
    /// ナノ秒精度のUnixタイムスタンプ
    pub nanos: u128,
}
```

---

## 6. HardwareControl

物理デバイスの制御（電源、クロック、ネットワーク）。

```rust
/// 物理デバイス制御
pub trait HardwareControl: Send + Sync {
    /// エラー型
    type Error: std::error::Error;

    /// クロック周波数を変更
    fn set_clock_frequency(&self, frequency_mhz: u32) -> Result<(), Self::Error>;

    /// ネットワークを遮断
    fn disconnect_network(&self) -> Result<(), Self::Error>;

    /// ネットワークを再接続
    fn reconnect_network(&self) -> Result<(), Self::Error>;

    /// 電源を遮断
    fn cut_power(&self) -> Result<(), Self::Error>;

    /// メモリ帯域を制限
    fn limit_memory_bandwidth(&self, percentage: u8) -> Result<(), Self::Error>;

    /// この実装がハードウェアに裏打ちされているか
    fn is_hardware_backed(&self) -> bool;
}
```

---

## 7. DataDiode

物理的な一方向通信。

```rust
/// 一方向通信（データダイオード）
pub trait DataDiode: Send + Sync {
    /// エラー型
    type Error: std::error::Error;

    /// データを受信（一方向のみ）
    ///
    /// 送信元へのフィードバックは物理的に不可能。
    fn receive(&self) -> Result<Vec<u8>, Self::Error>;

    /// 受信バッファの状態を取得
    fn buffer_status(&self) -> Result<BufferStatus, Self::Error>;

    /// この実装がハードウェアに裏打ちされているか
    fn is_hardware_backed(&self) -> bool;
}

pub struct BufferStatus {
    pub available_bytes: usize,
    pub total_capacity: usize,
    pub overflow_count: u64,
}
```

---

## モック実装のガイドライン

### 開発・テスト用モック

```rust
/// 全てのハードウェアトレイトのモック実装を提供
pub mod mock {
    pub struct MockImmutableStorage { /* ... */ }
    pub struct MockSecureRandom { /* ... */ }
    pub struct MockSecureExecution { /* ... */ }
    pub struct MockHsm { /* ... */ }
    pub struct MockTimeVerification { /* ... */ }
    pub struct MockHardwareControl { /* ... */ }
    pub struct MockDataDiode { /* ... */ }
}
```

### モック使用時の警告

```rust
/// モック実装を使用していることを検出して警告
pub fn warn_if_using_mocks(components: &[&dyn IsHardwareBacked]) {
    for component in components {
        if !component.is_hardware_backed() {
            eprintln!(
                "⚠️ WARNING: Using software mock for {:?}. \
                 This does NOT provide physical security guarantees.",
                component.name()
            );
        }
    }
}
```

---

## パートナー実装ガイド

### 実装すべきトレイト

| パートナー種別 | 実装トレイト |
|--------------|-------------|
| HSMベンダー | `ImmutableStorage`, `HardwareSecurityModule` |
| TEEプロバイダー | `SecureExecution` |
| QRNGベンダー | `SecureRandom` |
| クラウドプロバイダー | 複数（Nitro, CloudHSM等） |
| ネットワーク機器 | `DataDiode` |

### 認証要件

パートナー実装がCHINJU認証を受けるには：

1. 全ての必須トレイトメソッドを実装
2. `is_hardware_backed()` が `true` を返す
3. セキュリティ監査に合格
4. 公開レジストリへの登録
