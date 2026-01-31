# CHINJU Protocol Specification

**Version:** 0.1.0 (Draft)
**Date:** 2026-01-31

---

## 概要

CHINJUプロトコルは、AI安全・ガバナンスのための通信規約・仕様です。
「AIを人間が止められる」ことを、物理法則レベルで保証します。

---

## ドキュメント構成

### コア仕様

| 文書番号 | タイトル | 概要 |
|---------|---------|------|
| [CHINJU-000](./CHINJU-000-TRUST_MODEL.md) | Trust Model | 信頼の起点、ブートストラップ、鍵ライフサイクル |
| [CHINJU-001](./CHINJU-001-ARCHITECTURE.md) | Architecture | 全体構成、レイヤー、コンポーネント配置 |
| [CHINJU-002](./CHINJU-002-DATA_FORMATS.md) | Data Formats | Protobuf/JSON Schema によるデータ定義 |
| [CHINJU-003](./CHINJU-003-STATE_MACHINES.md) | State Machines | 状態遷移、ライフサイクル |
| [CHINJU-004](./CHINJU-004-API.md) | API Specification | gRPC サービス定義 |
| [CHINJU-005](./CHINJU-005-HARDWARE.md) | Hardware Requirements | ハードウェア要件、セキュリティレベル |

### 補足文書（予定）

| 文書番号 | タイトル | 概要 |
|---------|---------|------|
| CHINJU-006 | Versioning | バージョニング規則、互換性ポリシー |
| CHINJU-007 | Conformance | 適合性テスト、認定プロセス |
| CHINJU-008 | Migration | バージョン間移行手順 |
| CHINJU-009 | Security | セキュリティ考慮事項、脅威モデル |

---

## クイックスタート

### 実装者向け

1. **理解する**: CHINJU-000 (Trust Model) と CHINJU-001 (Architecture) を読む
2. **設計する**: CHINJU-002 (Data Formats) と CHINJU-003 (State Machines) を参照
3. **実装する**: CHINJU-004 (API) の gRPC サービスを実装
4. **ハードウェア**: CHINJU-005 でセキュリティレベルを選択

### 運用者向け

1. **要件確認**: CHINJU-005 でハードウェア要件を確認
2. **レベル選択**: L0-L4 からセキュリティレベルを選択
3. **デプロイ**: CHINJU-001 のデプロイメントモデルを参照

---

## プロトコルスタック

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer 5: Application                                            │
│  ────────────────────────────────────────────────────────────── │
│  ユーザーアプリケーション、AI モデル                             │
├─────────────────────────────────────────────────────────────────┤
│  Layer 4: Policy                      [CHINJU-002, 003]          │
│  ────────────────────────────────────────────────────────────── │
│  ポリシーパック、ルールエンジン                                  │
├─────────────────────────────────────────────────────────────────┤
│  Layer 3: Control                     [CHINJU-001, 004]          │
│  ────────────────────────────────────────────────────────────── │
│  HCAL, トークン, 監査, Sidecar                                   │
├─────────────────────────────────────────────────────────────────┤
│  Layer 2: Trust                       [CHINJU-000]               │
│  ────────────────────────────────────────────────────────────── │
│  権限分散, 人間証明, 閾値署名                                    │
├─────────────────────────────────────────────────────────────────┤
│  Layer 1: Crypto                      [CHINJU-002, 005]          │
│  ────────────────────────────────────────────────────────────── │
│  署名, ハッシュ, 秘密分散                                        │
├─────────────────────────────────────────────────────────────────┤
│  Layer 0: Hardware                    [CHINJU-005]               │
│  ────────────────────────────────────────────────────────────── │
│  HSM, TPM, TEE, OTP, QRNG                                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## 主要コンポーネント

### ランタイム

| コンポーネント | 説明 | 参照 |
|--------------|------|------|
| **CHINJU Wallet** | ユーザーデバイス上のクライアント | CHINJU-001 §3.2 |
| **CHINJU Sidecar** | AI モデルの前段プロキシ | CHINJU-001 §3.3 |
| **CHINJU Registry** | 分散台帳・正本管理 | CHINJU-001 §3.4 |

### データ構造

| 構造 | 説明 | 参照 |
|------|------|------|
| **Human Credential** | 人間証明書 | CHINJU-002 §3 |
| **Survival Token** | 生存トークン | CHINJU-002 §4 |
| **Policy Pack** | ポリシーパック | CHINJU-002 §5 |

### API サービス

| サービス | 説明 | 参照 |
|---------|------|------|
| **CredentialService** | 証明書管理 | CHINJU-004 §3 |
| **TokenService** | トークン管理 | CHINJU-004 §4 |
| **PolicyService** | ポリシー管理 | CHINJU-004 §5 |
| **AIGatewayService** | AI リクエスト仲介 | CHINJU-004 §6 |
| **AuditService** | 監査ログ | CHINJU-004 §7 |

---

## セキュリティレベル

| レベル | 名称 | 用途 | 主要ハードウェア |
|--------|------|------|-----------------|
| L0 | Development | 開発 | エミュレーション |
| L1 | Basic | 小規模 | Software TPM |
| L2 | Standard | 一般 | TPM 2.0 + TEE |
| L3 | Enterprise | 大規模 | HSM + TPM + TEE |
| L4 | Critical | 重要インフラ | HSM + QRNG + Data Diode |

詳細: CHINJU-005

---

## ステータス

```
[■■□□□□□□□□] 20% - Draft

完了:
  ✅ Trust Model (CHINJU-000)
  ✅ Architecture (CHINJU-001)
  ✅ Data Formats (CHINJU-002)
  ✅ State Machines (CHINJU-003)
  ✅ API Specification (CHINJU-004)
  ✅ Hardware Requirements (CHINJU-005)

未着手:
  ⬜ Versioning (CHINJU-006)
  ⬜ Conformance (CHINJU-007)
  ⬜ Migration (CHINJU-008)
  ⬜ Security (CHINJU-009)
  ⬜ Reference Implementation
  ⬜ Test Suite
```

---

## 貢献

プロトコルの変更は RFC プロセスを通じて行います。
詳細: CHINJU-000 §6

---

## ライセンス

[TBD]

---

## 関連文書

- [特許明細書 (sample/spec/)](../sample/spec/)
- [ハードウェア参照実装 (sample/hardware/)](../sample/hardware/)
