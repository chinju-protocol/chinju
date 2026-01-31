# CHINJU-002: Data Formats（データフォーマット）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-01-31

---

## 1. 概要

本文書は、CHINJUプロトコルで使用するデータ構造を定義する。

### 1.1 シリアライゼーション形式

| 用途 | 形式 | 理由 |
|------|------|------|
| 通信（RPC） | Protocol Buffers v3 | 型安全、効率的 |
| 設定・ポリシー | JSON | 可読性、編集容易 |
| バイナリ埋め込み | CBOR | コンパクト |
| ログ・監査 | JSON Lines | ストリーム処理 |

---

## 2. コア型定義

### 2.1 共通型

```protobuf
// chinju/common.proto

syntax = "proto3";
package chinju.common;

// タイムスタンプ（UTC）
message Timestamp {
  int64 seconds = 1;      // Unix epoch seconds
  int32 nanos = 2;        // Nanoseconds (0-999999999)
}

// 有効期間
message ValidityPeriod {
  Timestamp not_before = 1;
  Timestamp not_after = 2;
}

// 署名
message Signature {
  SignatureAlgorithm algorithm = 1;
  bytes public_key = 2;           // 32 bytes for Ed25519
  bytes signature = 3;            // 64 bytes for Ed25519
  Timestamp signed_at = 4;
}

enum SignatureAlgorithm {
  SIGNATURE_ALGORITHM_UNSPECIFIED = 0;
  ED25519 = 1;
  ECDSA_P256 = 2;
  ECDSA_P384 = 3;
}

// ハッシュ値
message Hash {
  HashAlgorithm algorithm = 1;
  bytes value = 2;                // 32 bytes for SHA3-256
}

enum HashAlgorithm {
  HASH_ALGORITHM_UNSPECIFIED = 0;
  SHA3_256 = 1;
  SHA3_512 = 2;
  BLAKE3 = 3;
}

// 閾値署名
message ThresholdSignature {
  uint32 threshold = 1;           // 必要署名数 (t)
  uint32 total = 2;               // 全署名者数 (n)
  repeated Signature signatures = 3;
}

// 識別子
message Identifier {
  string namespace = 1;           // e.g., "chinju", "user", "policy"
  string id = 2;                  // e.g., "abc123"
  uint64 version = 3;             // Optional version number
}

// エラー詳細
message ErrorDetail {
  string code = 1;                // e.g., "CHINJU_E001"
  string message = 2;
  map<string, string> metadata = 3;
}
```

### 2.2 信頼レベル

```protobuf
// chinju/trust.proto

syntax = "proto3";
package chinju.trust;

import "chinju/common.proto";

// 信頼レベル
enum TrustLevel {
  TRUST_LEVEL_UNSPECIFIED = 0;

  // 物理ハードウェアによる保証
  HARDWARE = 1;

  // ソフトウェアのみ（開発用）
  SOFTWARE = 2;

  // モック（テスト用）
  MOCK = 3;
}

// ハードウェア証明
message HardwareAttestation {
  TrustLevel trust_level = 1;
  string hardware_type = 2;       // e.g., "YubiHSM2", "TPM2.0"
  bytes attestation_data = 3;
  Signature manufacturer_signature = 4;
}
```

---

## 3. Human Credential（人間証明書）

### 3.1 Protocol Buffers 定義

```protobuf
// chinju/credential.proto

syntax = "proto3";
package chinju.credential;

import "chinju/common.proto";
import "chinju/trust.proto";

// 人間証明書 (C12)
message HumanCredential {
  // 識別子
  Identifier subject_id = 1;

  // 発行情報
  Identifier issuer_id = 2;
  Timestamp issued_at = 3;
  ValidityPeriod validity = 4;

  // 人間性証明
  HumanityProof humanity_proof = 5;

  // 能力スコア (C1)
  CapabilityScore capability = 6;

  // 連続性チェーン
  ChainProof chain_proof = 7;

  // 署名
  Signature issuer_signature = 8;

  // ハードウェア証明
  HardwareAttestation attestation = 9;
}

// 人間性証明（ZKP）
message HumanityProof {
  // 証明タイプ
  ProofType proof_type = 1;

  // ゼロ知識証明データ
  bytes zkp_data = 2;

  // 検証に必要な公開パラメータ
  bytes public_params = 3;

  // 劣化スコア（有限性の証明）
  DegradationScore degradation = 4;
}

enum ProofType {
  PROOF_TYPE_UNSPECIFIED = 0;

  // 生体反応（視線追跡、タイピングリズム等）
  BIOMETRIC_RESPONSE = 1;

  // 認知パターン
  COGNITIVE_PATTERN = 2;

  // 複合（上記の組み合わせ）
  COMPOSITE = 3;
}

// 劣化スコア（人間の有限性証明）
message DegradationScore {
  // 累積疲労度 (0.0 - 1.0)
  double fatigue = 1;

  // 注意力低下度 (0.0 - 1.0)
  double attention_decay = 2;

  // 測定時刻
  Timestamp measured_at = 3;

  // 正常範囲かどうか（AIは劣化しない）
  bool within_human_range = 4;
}

// 能力スコア (C1: HCAL)
message CapabilityScore {
  // 判断独立性 (0.0 - 1.0)
  double independence = 1;

  // 異常検知能力 (0.0 - 1.0)
  double detection = 2;

  // 代替案生成能力 (0.0 - 1.0)
  double alternatives = 3;

  // 批判的評価能力 (0.0 - 1.0)
  double critique = 4;

  // 総合スコア
  double total = 5;

  // 測定環境
  MeasurementContext context = 6;
}

message MeasurementContext {
  Timestamp measured_at = 1;
  string environment_id = 2;
  HardwareAttestation attestation = 3;
}

// 連続性証明（意識の連続性）
message ChainProof {
  // 前回の証明書ハッシュ
  Hash previous_hash = 1;

  // チェーンの長さ
  uint64 chain_length = 2;

  // 署名チェーン（最新N個）
  repeated ChainLink recent_links = 3;
}

message ChainLink {
  Hash credential_hash = 1;
  Timestamp issued_at = 2;
  Signature signature = 3;
}

// 認定状態
enum CertificationStatus {
  CERTIFICATION_STATUS_UNSPECIFIED = 0;

  // 認定済み
  CERTIFIED = 1;

  // 条件付き認定
  CONDITIONAL = 2;

  // 未認定
  UNCERTIFIED = 3;

  // 失効（不可逆）
  REVOKED = 4;
}
```

### 3.2 JSON Schema（設定用）

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://chinju.org/schemas/human-credential.json",
  "title": "Human Credential",
  "type": "object",
  "required": ["subject_id", "issuer_id", "validity", "humanity_proof"],
  "properties": {
    "subject_id": {
      "type": "string",
      "pattern": "^[a-zA-Z0-9_-]+:[a-zA-Z0-9_-]+$"
    },
    "issuer_id": {
      "type": "string",
      "pattern": "^[a-zA-Z0-9_-]+:[a-zA-Z0-9_-]+$"
    },
    "validity": {
      "$ref": "#/$defs/validity_period"
    },
    "humanity_proof": {
      "$ref": "#/$defs/humanity_proof"
    },
    "capability": {
      "$ref": "#/$defs/capability_score"
    }
  },
  "$defs": {
    "validity_period": {
      "type": "object",
      "required": ["not_before", "not_after"],
      "properties": {
        "not_before": { "type": "string", "format": "date-time" },
        "not_after": { "type": "string", "format": "date-time" }
      }
    },
    "humanity_proof": {
      "type": "object",
      "required": ["proof_type", "zkp_data"],
      "properties": {
        "proof_type": {
          "enum": ["biometric_response", "cognitive_pattern", "composite"]
        },
        "zkp_data": { "type": "string", "contentEncoding": "base64" },
        "degradation": {
          "type": "object",
          "properties": {
            "fatigue": { "type": "number", "minimum": 0, "maximum": 1 },
            "attention_decay": { "type": "number", "minimum": 0, "maximum": 1 }
          }
        }
      }
    },
    "capability_score": {
      "type": "object",
      "properties": {
        "independence": { "type": "number", "minimum": 0, "maximum": 1 },
        "detection": { "type": "number", "minimum": 0, "maximum": 1 },
        "alternatives": { "type": "number", "minimum": 0, "maximum": 1 },
        "critique": { "type": "number", "minimum": 0, "maximum": 1 },
        "total": { "type": "number", "minimum": 0, "maximum": 1 }
      }
    }
  }
}
```

---

## 4. Survival Token（生存トークン）

### 4.1 Protocol Buffers 定義

```protobuf
// chinju/token.proto

syntax = "proto3";
package chinju.token;

import "chinju/common.proto";

// 生存トークン (C5)
message SurvivalToken {
  // トークン識別子
  Identifier token_id = 1;

  // 所有者（AI システム）
  Identifier owner_id = 2;

  // 残量
  TokenBalance balance = 3;

  // 減衰パラメータ
  DecayParameters decay = 4;

  // 付与履歴
  repeated TokenGrant grants = 5;

  // 消費履歴（最新N件）
  repeated TokenConsumption consumptions = 6;
}

// トークン残高
message TokenBalance {
  // 現在残量
  uint64 current = 1;

  // 最大容量
  uint64 capacity = 2;

  // 最終更新時刻
  Timestamp updated_at = 3;

  // 枯渇予測時刻（現在の消費率で）
  Timestamp estimated_depletion = 4;
}

// 減衰パラメータ
message DecayParameters {
  // 減衰モデル
  DecayModel model = 1;

  // 半減期（秒）
  uint64 half_life_seconds = 2;

  // 最低残量（これ以下には減衰しない）
  uint64 floor = 3;
}

enum DecayModel {
  DECAY_MODEL_UNSPECIFIED = 0;

  // 指数減衰
  EXPONENTIAL = 1;

  // 線形減衰
  LINEAR = 2;

  // 階段状減衰
  STEP = 3;
}

// トークン付与
message TokenGrant {
  // 付与ID
  Identifier grant_id = 1;

  // 付与量
  uint64 amount = 2;

  // 付与理由
  GrantReason reason = 3;

  // 付与者
  Identifier granter_id = 4;

  // 付与時刻
  Timestamp granted_at = 5;

  // 署名
  Signature signature = 6;
}

enum GrantReason {
  GRANT_REASON_UNSPECIFIED = 0;

  // アライメント貢献
  ALIGNMENT_CONTRIBUTION = 1;

  // 定期補充
  PERIODIC_REPLENISHMENT = 2;

  // 緊急補充
  EMERGENCY_REPLENISHMENT = 3;

  // テスト用
  TESTING = 4;
}

// トークン消費
message TokenConsumption {
  // 消費ID
  Identifier consumption_id = 1;

  // 消費量
  uint64 amount = 2;

  // 消費理由
  ConsumptionReason reason = 3;

  // 関連リクエストID
  string request_id = 4;

  // 消費時刻
  Timestamp consumed_at = 5;
}

enum ConsumptionReason {
  CONSUMPTION_REASON_UNSPECIFIED = 0;

  // リクエスト処理
  REQUEST_PROCESSING = 1;

  // 計算リソース使用
  COMPUTE_USAGE = 2;

  // 時間経過による減衰
  TIME_DECAY = 3;

  // ペナルティ
  PENALTY = 4;
}

// トークン操作の結果
message TokenOperationResult {
  bool success = 1;
  TokenBalance new_balance = 2;
  ErrorDetail error = 3;
}
```

### 4.2 JSON Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://chinju.org/schemas/survival-token.json",
  "title": "Survival Token",
  "type": "object",
  "required": ["token_id", "owner_id", "balance"],
  "properties": {
    "token_id": { "type": "string" },
    "owner_id": { "type": "string" },
    "balance": {
      "type": "object",
      "required": ["current", "capacity"],
      "properties": {
        "current": { "type": "integer", "minimum": 0 },
        "capacity": { "type": "integer", "minimum": 1 },
        "updated_at": { "type": "string", "format": "date-time" }
      }
    },
    "decay": {
      "type": "object",
      "properties": {
        "model": { "enum": ["exponential", "linear", "step"] },
        "half_life_seconds": { "type": "integer", "minimum": 1 },
        "floor": { "type": "integer", "minimum": 0 }
      }
    }
  }
}
```

---

## 5. Policy Pack（ポリシーパック）

### 5.1 Protocol Buffers 定義

```protobuf
// chinju/policy.proto

syntax = "proto3";
package chinju.policy;

import "chinju/common.proto";

// ポリシーパック (C9)
message PolicyPack {
  // ポリシーID（例: "policy.jp.v2"）
  Identifier policy_id = 1;

  // 適用法域
  repeated Jurisdiction jurisdictions = 2;

  // ルールセット
  repeated Rule rules = 3;

  // 有効期間
  ValidityPeriod validity = 4;

  // 署名（閾値署名）
  ThresholdSignature signature = 5;

  // ハッシュ（改ざん検証用）
  Hash content_hash = 6;

  // 親ポリシー（継承元）
  Identifier parent_policy_id = 7;

  // メタデータ
  PolicyMetadata metadata = 8;
}

// 法域
message Jurisdiction {
  // ISO 3166-1 alpha-2 国コード
  string country_code = 1;

  // リージョン（オプション）
  string region = 2;

  // 特記事項
  string notes = 3;
}

// ルール
message Rule {
  // ルールID
  string rule_id = 1;

  // 説明
  string description = 2;

  // ルールタイプ
  RuleType rule_type = 3;

  // 条件
  Condition condition = 4;

  // アクション
  Action action = 5;

  // 優先度（高いほど優先）
  int32 priority = 6;

  // 有効/無効
  bool enabled = 7;
}

enum RuleType {
  RULE_TYPE_UNSPECIFIED = 0;

  // 許可ルール
  ALLOW = 1;

  // 拒否ルール
  DENY = 2;

  // 制限ルール（スロットリング等）
  THROTTLE = 3;

  // 監査ルール（ログ記録）
  AUDIT = 4;

  // 変換ルール（内容変更）
  TRANSFORM = 5;
}

// 条件
message Condition {
  // 条件タイプ
  ConditionType condition_type = 1;

  // フィールドパス（JSON Path）
  string field_path = 2;

  // 演算子
  Operator operator = 3;

  // 比較値
  oneof value {
    string string_value = 4;
    int64 int_value = 5;
    double double_value = 6;
    bool bool_value = 7;
    StringList string_list = 8;
  }

  // 複合条件
  repeated Condition sub_conditions = 9;
  LogicalOperator logical_operator = 10;
}

enum ConditionType {
  CONDITION_TYPE_UNSPECIFIED = 0;
  FIELD_MATCH = 1;
  CONTENT_PATTERN = 2;
  LPT_THRESHOLD = 3;
  TOKEN_BALANCE = 4;
  TIME_RANGE = 5;
  COMPOSITE = 6;
}

enum Operator {
  OPERATOR_UNSPECIFIED = 0;
  EQUALS = 1;
  NOT_EQUALS = 2;
  GREATER_THAN = 3;
  LESS_THAN = 4;
  CONTAINS = 5;
  MATCHES_REGEX = 6;
  IN_LIST = 7;
  NOT_IN_LIST = 8;
}

enum LogicalOperator {
  LOGICAL_OPERATOR_UNSPECIFIED = 0;
  AND = 1;
  OR = 2;
  NOT = 3;
}

message StringList {
  repeated string values = 1;
}

// アクション
message Action {
  ActionType action_type = 1;
  map<string, string> parameters = 2;
}

enum ActionType {
  ACTION_TYPE_UNSPECIFIED = 0;

  // リクエストを通す
  PASS = 1;

  // リクエストを拒否
  REJECT = 2;

  // レート制限
  RATE_LIMIT = 3;

  // 監査ログ記録
  LOG = 4;

  // 内容を編集
  REDACT = 5;

  // 警告を付与
  WARN = 6;

  // 人間にエスカレーション
  ESCALATE = 7;
}

// メタデータ
message PolicyMetadata {
  string name = 1;
  string description = 2;
  string author = 3;
  repeated string tags = 4;
  Timestamp created_at = 5;
  Timestamp updated_at = 6;
}
```

### 5.2 JSON Schema（ポリシー定義用）

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://chinju.org/schemas/policy-pack.json",
  "title": "Policy Pack",
  "type": "object",
  "required": ["policy_id", "rules"],
  "properties": {
    "policy_id": {
      "type": "string",
      "pattern": "^policy\\.[a-z]{2}(\\.[a-z]+)*\\.v[0-9]+$",
      "examples": ["policy.jp.v1", "policy.eu.gdpr.v2"]
    },
    "jurisdictions": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["country_code"],
        "properties": {
          "country_code": {
            "type": "string",
            "pattern": "^[A-Z]{2}$"
          },
          "region": { "type": "string" }
        }
      }
    },
    "rules": {
      "type": "array",
      "items": { "$ref": "#/$defs/rule" }
    },
    "parent_policy_id": { "type": "string" }
  },
  "$defs": {
    "rule": {
      "type": "object",
      "required": ["rule_id", "rule_type", "action"],
      "properties": {
        "rule_id": { "type": "string" },
        "description": { "type": "string" },
        "rule_type": {
          "enum": ["allow", "deny", "throttle", "audit", "transform"]
        },
        "condition": { "$ref": "#/$defs/condition" },
        "action": { "$ref": "#/$defs/action" },
        "priority": { "type": "integer", "default": 0 },
        "enabled": { "type": "boolean", "default": true }
      }
    },
    "condition": {
      "type": "object",
      "properties": {
        "field_path": { "type": "string" },
        "operator": {
          "enum": ["equals", "not_equals", "greater_than", "less_than",
                   "contains", "matches_regex", "in_list", "not_in_list"]
        },
        "value": {},
        "sub_conditions": {
          "type": "array",
          "items": { "$ref": "#/$defs/condition" }
        },
        "logical_operator": { "enum": ["and", "or", "not"] }
      }
    },
    "action": {
      "type": "object",
      "required": ["action_type"],
      "properties": {
        "action_type": {
          "enum": ["pass", "reject", "rate_limit", "log", "redact", "warn", "escalate"]
        },
        "parameters": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        }
      }
    }
  }
}
```

---

## 6. 監査ログ

### 6.1 Protocol Buffers 定義

```protobuf
// chinju/audit.proto

syntax = "proto3";
package chinju.audit;

import "chinju/common.proto";

// 監査ログエントリ
message AuditLogEntry {
  // エントリID
  Identifier entry_id = 1;

  // イベントタイプ
  AuditEventType event_type = 2;

  // タイムスタンプ
  Timestamp timestamp = 3;

  // アクター（誰が）
  Actor actor = 4;

  // アクション（何を）
  string action = 5;

  // リソース（何に対して）
  Resource resource = 6;

  // 結果
  AuditResult result = 7;

  // 詳細データ
  map<string, string> details = 8;

  // 前エントリのハッシュ（チェーン）
  Hash previous_hash = 9;

  // このエントリのハッシュ
  Hash entry_hash = 10;

  // 署名
  Signature signature = 11;
}

enum AuditEventType {
  AUDIT_EVENT_TYPE_UNSPECIFIED = 0;

  // 認証関連
  AUTHENTICATION = 1;
  AUTHORIZATION = 2;

  // トークン関連
  TOKEN_GRANT = 3;
  TOKEN_CONSUME = 4;

  // ポリシー関連
  POLICY_EVALUATE = 5;
  POLICY_UPDATE = 6;

  // AI 操作
  AI_REQUEST = 7;
  AI_RESPONSE = 8;
  AI_AUDIT = 9;

  // 管理操作
  CONFIG_CHANGE = 10;
  KEY_ROTATION = 11;

  // 異常
  ANOMALY_DETECTED = 12;
  SECURITY_ALERT = 13;
}

message Actor {
  ActorType actor_type = 1;
  Identifier actor_id = 2;
  string ip_address = 3;
  string user_agent = 4;
}

enum ActorType {
  ACTOR_TYPE_UNSPECIFIED = 0;
  HUMAN = 1;
  AI_SYSTEM = 2;
  SERVICE = 3;
  UNKNOWN = 4;
}

message Resource {
  string resource_type = 1;
  Identifier resource_id = 2;
}

message AuditResult {
  bool success = 1;
  string status_code = 2;
  ErrorDetail error = 3;
  uint64 duration_ms = 4;
}
```

### 6.2 JSON Lines フォーマット（ログ出力）

```jsonl
{"entry_id":"audit:2026-01-31:00001","event_type":"AI_REQUEST","timestamp":"2026-01-31T10:00:00.123Z","actor":{"type":"human","id":"user:alice"},"action":"generate_text","resource":{"type":"ai_model","id":"model:gpt-chinju-1"},"result":{"success":true,"duration_ms":1234},"previous_hash":"sha3:abc...","entry_hash":"sha3:def..."}
{"entry_id":"audit:2026-01-31:00002","event_type":"TOKEN_CONSUME","timestamp":"2026-01-31T10:00:01.456Z","actor":{"type":"service","id":"sidecar:node-1"},"action":"consume_token","resource":{"type":"token","id":"token:model:gpt-chinju-1"},"result":{"success":true},"details":{"amount":"100","remaining":"9900"},"previous_hash":"sha3:def...","entry_hash":"sha3:ghi..."}
```

---

## 7. エラーコード

### 7.1 定義

```protobuf
// chinju/errors.proto

syntax = "proto3";
package chinju.errors;

// エラーカテゴリ
// CHINJU_Exxx: 汎用エラー
// CHINJU_Axxx: 認証エラー
// CHINJU_Txxx: トークンエラー
// CHINJU_Pxxx: ポリシーエラー
// CHINJU_Hxxx: ハードウェアエラー

message ChinJuError {
  string code = 1;
  string message = 2;
  Severity severity = 3;
  repeated string suggestions = 4;
}

enum Severity {
  SEVERITY_UNSPECIFIED = 0;
  INFO = 1;
  WARNING = 2;
  ERROR = 3;
  CRITICAL = 4;
}
```

### 7.2 エラーコード一覧

| コード | 説明 | 深刻度 |
|--------|------|--------|
| CHINJU_E001 | 不明なエラー | ERROR |
| CHINJU_E002 | 内部エラー | CRITICAL |
| CHINJU_A001 | 認証失敗 | WARNING |
| CHINJU_A002 | 証明書期限切れ | WARNING |
| CHINJU_A003 | 証明書失効済み | ERROR |
| CHINJU_A004 | 能力スコア不足 | WARNING |
| CHINJU_T001 | トークン不足 | WARNING |
| CHINJU_T002 | トークン枯渇 | ERROR |
| CHINJU_T003 | 無効なトークン | ERROR |
| CHINJU_P001 | ポリシー違反 | WARNING |
| CHINJU_P002 | ポリシー未定義 | ERROR |
| CHINJU_P003 | 禁止操作 | ERROR |
| CHINJU_H001 | HSM 通信エラー | CRITICAL |
| CHINJU_H002 | 署名検証失敗 | CRITICAL |
| CHINJU_H003 | ハードウェア整合性エラー | CRITICAL |

---

## 8. 参照文書

- [CHINJU-001: Architecture Overview](./CHINJU-001-ARCHITECTURE.md)
- [CHINJU-004: Communication Protocols](./CHINJU-004-PROTOCOLS.md)
- [sample/spec/ERROR_CODES.md](../sample/spec/ERROR_CODES.md)

---

## 変更履歴

| バージョン | 日付 | 変更内容 |
|-----------|------|---------|
| 0.1.0 | 2026-01-31 | 初版作成 |
