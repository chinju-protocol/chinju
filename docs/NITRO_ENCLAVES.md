# AWS Nitro Enclaves 統合ガイド

CHINJU Protocol における AWS Nitro Enclaves の設定・運用ガイドです。

## 概要

Nitro Enclaves は、EC2 インスタンス上で隔離された実行環境を提供します。
CHINJU Protocol では L3（Enterprise）レベルのセキュリティを実現するために使用します。

```
EC2 Parent Instance              Nitro Enclave              AWS
+-------------------+           +------------------+        +-----+
| chinju-sidecar    |           | chinju-enclave   |        | KMS |
|                   |  vsock    |                  |        |     |
| GatewayService    | <-------> | VsockServer      |        |     |
|   ↓               |           |   ↓              |        |     |
| NitroService      |           | KeyManager       |        |     |
|                   |           | KmsClient -------|--proxy-|     |
+-------------------+           +------------------+        +-----+
```

## 前提条件

- Nitro Enclaves 対応の EC2 インスタンス（c5.xlarge 以上）
- Amazon Linux 2 または Ubuntu 20.04+
- `aws-nitro-enclaves-cli` インストール済み

## クイックスタート

### 1. EC2 インスタンスの準備

```bash
# Nitro Enclaves CLI インストール (Amazon Linux 2)
sudo amazon-linux-extras install aws-nitro-enclaves-cli -y
sudo yum install aws-nitro-enclaves-cli-devel -y

# ユーザーを ne グループに追加
sudo usermod -aG ne $USER

# Enclave allocator 設定
sudo systemctl enable nitro-enclaves-allocator.service
sudo systemctl start nitro-enclaves-allocator.service
```

### 2. Enclave イメージのビルド

```bash
# chinju-enclave をビルド
cd chinju-enclave
docker build -t chinju-enclave -f Dockerfile.enclave .

# EIF (Enclave Image File) を作成
nitro-cli build-enclave \
  --docker-uri chinju-enclave:latest \
  --output-file chinju-enclave.eif

# PCR 値を確認（KMS ポリシーに使用）
nitro-cli describe-eif --eif-path chinju-enclave.eif
```

### 3. Enclave の起動

```bash
# Enclave を起動
nitro-cli run-enclave \
  --eif-path chinju-enclave.eif \
  --memory 512 \
  --cpu-count 2

# CID を確認
nitro-cli describe-enclaves
# 出力例: [{"EnclaveCID": 16, "State": "RUNNING", ...}]
```

### 4. vsock-proxy の起動（KMS 連携用）

```bash
# セットアップスクリプトを使用
./scripts/setup-vsock-proxy.sh ap-northeast-1

# または手動で
vsock-proxy 8000 kms.ap-northeast-1.amazonaws.com 443
```

### 5. 通信テスト

```bash
# vsock 通信テスト
export CHINJU_NITRO_ENCLAVE_CID=16
./scripts/test-nitro-vsock.sh
```

## 本番環境の設定

### KMS キーの作成（Terraform）

```bash
cd deploy/terraform

# 設定ファイルを作成
cp terraform.tfvars.example terraform.tfvars

# PCR 値を設定（nitro-cli describe-eif の出力から）
vim terraform.tfvars
```

```hcl
# terraform.tfvars
aws_region = "ap-northeast-1"
environment = "prod"
enable_attestation = true

# Enclave の PCR 値
enclave_pcr0 = "ea85ddf01145538bc79518181e304f926365a971..."
enclave_pcr1 = "4b4d5b3661b3efc12920900c80e126e4ce783c52..."
enclave_pcr2 = "5fdad5964ebfa243bae2de4a86eb12c6ce6250a1..."
```

```bash
# KMS キーを作成
terraform init
terraform apply
```

### chinju-sidecar の設定

```bash
# 環境変数
export CHINJU_NITRO_ENABLED=true
export CHINJU_NITRO_ENCLAVE_CID=16
export CHINJU_NITRO_PORT=5000
export CHINJU_NITRO_DEBUG=false  # 本番では false

# Attestation ポリシー
export CHINJU_ENCLAVE_PCR0="ea85ddf01145538bc79518181e304f926365a971..."
export CHINJU_ENCLAVE_ALLOW_DEBUG=false
```

### chinju-enclave の設定（Enclave 内）

```bash
# KMS 設定
export AWS_KMS_KEY_ID="arn:aws:kms:ap-northeast-1:123456789012:key/..."
export AWS_REGION="ap-northeast-1"

# vsock-proxy 設定
export VSOCK_PROXY_CID=3
export VSOCK_PROXY_PORT=8000
```

## API の使用方法

### Rust (chinju-sidecar 内)

```rust
use chinju_sidecar::services::{ContainmentConfig, GatewayService};

// Nitro Enclave 有効で Gateway を作成
let config = ContainmentConfig::production_with_nitro(16); // CID=16

let gateway = GatewayService::with_containment(
    token_service,
    credential_service,
    policy_engine,
    Some(audit_logger),
    Some(openai_client),
    config,
).await;

// Enclave 経由で署名
let (signature, public_key) = gateway.secure_sign("key-id", data).await?;

// Enclave 経由でデータをシール
let sealed_data = gateway.secure_seal(secret_data).await?;

// Enclave 経由でデータをアンシール
let plaintext = gateway.secure_unseal(&sealed_data).await?;

// Attestation ドキュメントを取得
let attestation = gateway.get_attestation(challenge, user_data).await?;
```

### 直接 NitroService を使用

```rust
use chinju_sidecar::services::{NitroService, NitroServiceConfig};

let config = NitroServiceConfig {
    enabled: true,
    cid: Some(16),
    port: 5000,
    debug: false,
    timeout_ms: 5000,
};

let mut service = NitroService::new(config);
service.connect().await?;

// ヘルスチェック
if service.is_healthy().await {
    println!("Enclave is healthy");
}

// 鍵ペア生成
let (key_id, public_key) = service.generate_key_pair("Ed25519", "my-key").await?;
```

## トラブルシューティング

### Enclave が起動しない

```bash
# メモリ割り当てを確認
cat /etc/nitro_enclaves/allocator.yaml

# hugepages を確認
cat /proc/meminfo | grep Huge

# ログを確認
journalctl -u nitro-enclaves-allocator
```

### vsock 接続エラー

```bash
# Enclave が起動しているか確認
nitro-cli describe-enclaves

# CID が正しいか確認
export CHINJU_NITRO_ENCLAVE_CID=$(nitro-cli describe-enclaves | jq -r '.[0].EnclaveCID')

# vsock-proxy が起動しているか確認
pgrep -a vsock-proxy
```

### KMS アクセスエラー

```bash
# IAM ロールを確認
aws sts get-caller-identity

# KMS キーポリシーを確認
aws kms get-key-policy --key-id <key-id> --policy-name default

# PCR 値が一致しているか確認
nitro-cli describe-enclaves | jq '.[0].Measurements'
```

## セキュリティベストプラクティス

1. **本番環境では必ず Attestation を有効化**
   - `CHINJU_NITRO_DEBUG=false`
   - `enable_attestation=true` (Terraform)

2. **PCR 値を CI/CD で管理**
   - EIF ビルド時に PCR 値を記録
   - KMS ポリシーを自動更新

3. **最小権限の原則**
   - Enclave ホストの IAM ロールは必要最小限に
   - KMS キーへのアクセスは Attestation 必須

4. **監査ログの有効化**
   - CloudTrail で KMS API 呼び出しを記録
   - chinju-sidecar の監査ログを有効化

## 関連ファイル

| ファイル | 説明 |
|---------|------|
| `chinju-core/src/hardware/nitro/` | Nitro クライアント実装 |
| `chinju-enclave/` | Enclave アプリケーション |
| `chinju-sidecar/src/services/nitro.rs` | NitroService ラッパー |
| `deploy/terraform/nitro_kms.tf` | KMS Terraform 設定 |
| `scripts/setup-vsock-proxy.sh` | vsock-proxy セットアップ |
| `scripts/test-nitro-vsock.sh` | 通信テストスクリプト |

## 参考資料

- [AWS Nitro Enclaves Documentation](https://docs.aws.amazon.com/enclaves/latest/user/)
- [KMS Condition Keys for Nitro Enclaves](https://docs.aws.amazon.com/kms/latest/developerguide/policy-conditions.html#conditions-nitro-enclaves)
- [Nitro Enclaves SDK](https://github.com/aws/aws-nitro-enclaves-sdk-c)
