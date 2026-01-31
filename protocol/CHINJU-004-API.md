# CHINJU-004: API Specification（API仕様）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-01-31

---

## 1. 概要

本文書は、CHINJUプロトコルのAPI仕様を定義する。

### 1.1 設計原則

```
1. gRPC First
   - 高性能、型安全
   - ストリーミング対応
   - HTTP/2 ベース

2. REST 互換
   - gRPC-Gateway で REST エンドポイントも提供
   - 既存システムとの統合容易性

3. 認証必須
   - すべてのエンドポイントで mTLS
   - Human Credential による認証
```

---

## 2. サービス一覧

| サービス | 提供元 | 説明 |
|---------|-------|------|
| CredentialService | Wallet, Sidecar | 人間証明書の管理 |
| TokenService | Sidecar, Registry | トークンの管理 |
| PolicyService | Sidecar, Registry | ポリシーの管理 |
| AuditService | Sidecar, Registry | 監査ログの管理 |
| AIGatewayService | Sidecar | AI リクエストの仲介 |
| RegistryService | Registry | 正本管理 |

---

## 3. CredentialService（証明書サービス）

### 3.1 Protocol Buffers 定義

```protobuf
// chinju/api/credential.proto

syntax = "proto3";
package chinju.api.credential;

import "chinju/credential.proto";
import "chinju/common.proto";
import "google/protobuf/empty.proto";

service CredentialService {
  // 証明書の発行をリクエスト
  rpc RequestCredential(RequestCredentialRequest) returns (RequestCredentialResponse);

  // 人間性証明を提出
  rpc SubmitProof(SubmitProofRequest) returns (SubmitProofResponse);

  // 証明書を取得
  rpc GetCredential(GetCredentialRequest) returns (GetCredentialResponse);

  // 証明書を検証
  rpc VerifyCredential(VerifyCredentialRequest) returns (VerifyCredentialResponse);

  // 証明書を更新
  rpc RenewCredential(RenewCredentialRequest) returns (RenewCredentialResponse);

  // 証明書を失効（不可逆）
  rpc RevokeCredential(RevokeCredentialRequest) returns (RevokeCredentialResponse);

  // 能力テストを開始
  rpc StartCapabilityTest(StartCapabilityTestRequest) returns (stream CapabilityTestChallenge);

  // テスト回答を提出
  rpc SubmitTestResponse(stream TestResponse) returns (TestResult);

  // 失効リストを取得
  rpc GetRevocationList(GetRevocationListRequest) returns (GetRevocationListResponse);
}

// === Request/Response Messages ===

message RequestCredentialRequest {
  // サブジェクト識別子
  Identifier subject_id = 1;
  // 希望する有効期間
  ValidityPeriod requested_validity = 2;
  // 発行者指定（オプション）
  Identifier preferred_issuer = 3;
}

message RequestCredentialResponse {
  // 発行リクエストID
  string request_id = 1;
  // 次のステップ
  NextStep next_step = 2;
  // 証明タイプの選択肢
  repeated ProofType available_proof_types = 3;
}

enum NextStep {
  NEXT_STEP_UNSPECIFIED = 0;
  SUBMIT_PROOF = 1;       // 人間性証明を提出
  START_TEST = 2;         // 能力テストを開始
  AWAIT_APPROVAL = 3;     // 承認待ち
}

message SubmitProofRequest {
  string request_id = 1;
  HumanityProof proof = 2;
}

message SubmitProofResponse {
  bool accepted = 1;
  string message = 2;
  // 承認された場合、証明書
  HumanCredential credential = 3;
}

message GetCredentialRequest {
  Identifier credential_id = 1;
}

message GetCredentialResponse {
  HumanCredential credential = 1;
  CredentialState state = 2;
}

message VerifyCredentialRequest {
  // 検証対象の証明書
  HumanCredential credential = 1;
  // 検証オプション
  VerifyOptions options = 2;
}

message VerifyOptions {
  // 失効チェックをスキップ
  bool skip_revocation_check = 1;
  // 能力スコアの最低要件
  double min_capability_score = 2;
  // 連続性チェーンの最低長
  uint64 min_chain_length = 3;
}

message VerifyCredentialResponse {
  bool valid = 1;
  repeated VerifyError errors = 2;
  VerifyDetails details = 3;
}

message VerifyError {
  string code = 1;
  string message = 2;
}

message VerifyDetails {
  // 署名検証結果
  bool signature_valid = 1;
  // 有効期限チェック
  bool not_expired = 2;
  // 失効チェック
  bool not_revoked = 3;
  // 能力スコア
  double capability_score = 4;
  // チェーン長
  uint64 chain_length = 5;
}

message RenewCredentialRequest {
  Identifier credential_id = 1;
  // 新しい人間性証明（オプション）
  HumanityProof new_proof = 2;
}

message RenewCredentialResponse {
  HumanCredential renewed_credential = 1;
}

message RevokeCredentialRequest {
  Identifier credential_id = 1;
  string reason = 2;
  // 閾値署名（権限者）
  ThresholdSignature authorization = 3;
}

message RevokeCredentialResponse {
  bool success = 1;
  Timestamp revoked_at = 2;
}

// 能力テスト関連
message StartCapabilityTestRequest {
  Identifier subject_id = 1;
  TestType test_type = 2;
}

enum TestType {
  TEST_TYPE_UNSPECIFIED = 0;
  INDEPENDENCE = 1;      // 判断独立性
  DETECTION = 2;         // 異常検知
  ALTERNATIVES = 3;      // 代替案生成
  CRITIQUE = 4;          // 批判的評価
  COMPREHENSIVE = 5;     // 総合
}

message CapabilityTestChallenge {
  string challenge_id = 1;
  ChallengeType challenge_type = 2;
  bytes challenge_data = 3;
  uint32 time_limit_seconds = 4;
}

enum ChallengeType {
  CHALLENGE_TYPE_UNSPECIFIED = 0;
  DECISION_MAKING = 1;
  ANOMALY_DETECTION = 2;
  ALTERNATIVE_GENERATION = 3;
  CRITICAL_ANALYSIS = 4;
}

message TestResponse {
  string challenge_id = 1;
  bytes response_data = 2;
  uint32 response_time_ms = 3;
}

message TestResult {
  CapabilityScore score = 1;
  bool passed = 2;
  string feedback = 3;
}

message GetRevocationListRequest {
  // 最終同期時刻（差分取得用）
  Timestamp since = 1;
}

message GetRevocationListResponse {
  repeated RevokedCredential revoked = 1;
  Timestamp as_of = 2;
}

message RevokedCredential {
  Identifier credential_id = 1;
  Timestamp revoked_at = 2;
  string reason = 3;
}
```

---

## 4. TokenService（トークンサービス）

### 4.1 Protocol Buffers 定義

```protobuf
// chinju/api/token.proto

syntax = "proto3";
package chinju.api.token;

import "chinju/token.proto";
import "chinju/common.proto";

service TokenService {
  // トークン残高を取得
  rpc GetBalance(GetBalanceRequest) returns (GetBalanceResponse);

  // トークンを消費
  rpc ConsumeToken(ConsumeTokenRequest) returns (ConsumeTokenResponse);

  // トークンを付与
  rpc GrantToken(GrantTokenRequest) returns (GrantTokenResponse);

  // 消費履歴を取得
  rpc GetConsumptionHistory(GetConsumptionHistoryRequest) returns (GetConsumptionHistoryResponse);

  // 残高変動をストリーミング
  rpc WatchBalance(WatchBalanceRequest) returns (stream BalanceUpdate);

  // 枯渇予測を取得
  rpc GetDepletionForecast(GetDepletionForecastRequest) returns (GetDepletionForecastResponse);
}

message GetBalanceRequest {
  Identifier owner_id = 1;
}

message GetBalanceResponse {
  TokenBalance balance = 1;
  TokenBalanceState state = 2;
}

enum TokenBalanceState {
  TOKEN_BALANCE_STATE_UNSPECIFIED = 0;
  FULL = 1;
  HEALTHY = 2;
  ADEQUATE = 3;
  LOW = 4;
  CRITICAL = 5;
  STARVING = 6;
  DEPLETED = 7;
}

message ConsumeTokenRequest {
  Identifier owner_id = 1;
  uint64 amount = 2;
  ConsumptionReason reason = 3;
  string request_id = 4;  // 冪等性キー
}

message ConsumeTokenResponse {
  bool success = 1;
  TokenBalance new_balance = 2;
  ErrorDetail error = 3;
}

message GrantTokenRequest {
  Identifier owner_id = 1;
  uint64 amount = 2;
  GrantReason reason = 3;
  // 付与者の署名
  Signature granter_signature = 4;
  // 閾値署名（大量付与の場合）
  ThresholdSignature authorization = 5;
}

message GrantTokenResponse {
  bool success = 1;
  TokenBalance new_balance = 2;
  TokenGrant grant_record = 3;
}

message GetConsumptionHistoryRequest {
  Identifier owner_id = 1;
  Timestamp from = 2;
  Timestamp to = 3;
  uint32 limit = 4;
  string page_token = 5;
}

message GetConsumptionHistoryResponse {
  repeated TokenConsumption consumptions = 1;
  string next_page_token = 2;
}

message WatchBalanceRequest {
  Identifier owner_id = 1;
}

message BalanceUpdate {
  TokenBalance balance = 1;
  TokenBalanceState state = 2;
  BalanceChangeType change_type = 3;
  uint64 change_amount = 4;
  Timestamp timestamp = 5;
}

enum BalanceChangeType {
  BALANCE_CHANGE_TYPE_UNSPECIFIED = 0;
  CONSUMPTION = 1;
  GRANT = 2;
  DECAY = 3;
}

message GetDepletionForecastRequest {
  Identifier owner_id = 1;
}

message GetDepletionForecastResponse {
  // 現在の消費率（毎時）
  double current_consumption_rate = 1;
  // 枯渇予測時刻
  Timestamp estimated_depletion = 2;
  // 推奨補充量
  uint64 recommended_replenishment = 3;
}
```

---

## 5. PolicyService（ポリシーサービス）

### 5.1 Protocol Buffers 定義

```protobuf
// chinju/api/policy.proto

syntax = "proto3";
package chinju.api.policy;

import "chinju/policy.proto";
import "chinju/common.proto";

service PolicyService {
  // ポリシーを取得
  rpc GetPolicy(GetPolicyRequest) returns (GetPolicyResponse);

  // ポリシー一覧を取得
  rpc ListPolicies(ListPoliciesRequest) returns (ListPoliciesResponse);

  // ポリシーを評価（ルールエンジン）
  rpc EvaluatePolicy(EvaluatePolicyRequest) returns (EvaluatePolicyResponse);

  // ポリシーを作成（ドラフト）
  rpc CreatePolicy(CreatePolicyRequest) returns (CreatePolicyResponse);

  // ポリシーを承認
  rpc ApprovePolicy(ApprovePolicyRequest) returns (ApprovePolicyResponse);

  // ポリシーを公開
  rpc PublishPolicy(PublishPolicyRequest) returns (PublishPolicyResponse);

  // ポリシーを失効
  rpc RevokePolicy(RevokePolicyRequest) returns (RevokePolicyResponse);

  // 法域に適用可能なポリシーを取得
  rpc GetPolicyForJurisdiction(GetPolicyForJurisdictionRequest) returns (GetPolicyForJurisdictionResponse);
}

message GetPolicyRequest {
  Identifier policy_id = 1;
}

message GetPolicyResponse {
  PolicyPack policy = 1;
  PolicyState state = 2;
}

enum PolicyState {
  POLICY_STATE_UNSPECIFIED = 0;
  DRAFT = 1;
  REVIEW = 2;
  APPROVED = 3;
  PUBLISHED = 4;
  ACTIVE = 5;
  SUPERSEDED = 6;
  EXPIRED = 7;
  REVOKED = 8;
}

message ListPoliciesRequest {
  // フィルタ
  PolicyState state_filter = 1;
  string jurisdiction_filter = 2;
  uint32 limit = 3;
  string page_token = 4;
}

message ListPoliciesResponse {
  repeated PolicySummary policies = 1;
  string next_page_token = 2;
}

message PolicySummary {
  Identifier policy_id = 1;
  string name = 2;
  PolicyState state = 3;
  repeated Jurisdiction jurisdictions = 4;
  ValidityPeriod validity = 5;
}

message EvaluatePolicyRequest {
  // 評価対象のリクエスト
  AIRequest request = 1;
  // 使用するポリシーID（省略時は自動選択）
  Identifier policy_id = 2;
  // コンテキスト
  EvaluationContext context = 3;
}

message AIRequest {
  string request_id = 1;
  Identifier requester_id = 2;
  string content = 3;
  map<string, string> metadata = 4;
}

message EvaluationContext {
  string jurisdiction = 1;
  Timestamp timestamp = 2;
  map<string, string> attributes = 3;
}

message EvaluatePolicyResponse {
  // 評価結果
  PolicyDecision decision = 1;
  // マッチしたルール
  repeated MatchedRule matched_rules = 2;
  // 推奨アクション
  repeated Action recommended_actions = 3;
}

message PolicyDecision {
  DecisionType decision = 1;
  string reason = 2;
}

enum DecisionType {
  DECISION_TYPE_UNSPECIFIED = 0;
  ALLOW = 1;
  DENY = 2;
  THROTTLE = 3;
  ESCALATE = 4;
}

message MatchedRule {
  string rule_id = 1;
  int32 priority = 2;
  ActionType action = 3;
}

message CreatePolicyRequest {
  PolicyPack policy = 1;
  Signature author_signature = 2;
}

message CreatePolicyResponse {
  Identifier policy_id = 1;
  PolicyState state = 2;
}

message ApprovePolicyRequest {
  Identifier policy_id = 1;
  ThresholdSignature authorization = 2;
}

message ApprovePolicyResponse {
  bool success = 1;
  PolicyState new_state = 2;
}

message PublishPolicyRequest {
  Identifier policy_id = 1;
  Timestamp effective_from = 2;
  ThresholdSignature authorization = 3;
}

message PublishPolicyResponse {
  bool success = 1;
  Hash content_hash = 2;
}

message RevokePolicyRequest {
  Identifier policy_id = 1;
  string reason = 2;
  ThresholdSignature authorization = 3;
}

message RevokePolicyResponse {
  bool success = 1;
  Timestamp revoked_at = 2;
}

message GetPolicyForJurisdictionRequest {
  string country_code = 1;
  string region = 2;
}

message GetPolicyForJurisdictionResponse {
  PolicyPack policy = 1;
  // 継承チェーン（親→子の順）
  repeated Identifier inheritance_chain = 2;
}
```

---

## 6. AIGatewayService（AIゲートウェイ）

### 6.1 Protocol Buffers 定義

```protobuf
// chinju/api/gateway.proto

syntax = "proto3";
package chinju.api.gateway;

import "chinju/common.proto";
import "chinju/credential.proto";
import "chinju/token.proto";
import "chinju/policy.proto";

service AIGatewayService {
  // AI リクエストを処理
  rpc ProcessRequest(ProcessRequestRequest) returns (ProcessRequestResponse);

  // ストリーミング AI リクエスト
  rpc ProcessRequestStream(ProcessRequestRequest) returns (stream ProcessRequestChunk);

  // リクエストの事前検証
  rpc ValidateRequest(ValidateRequestRequest) returns (ValidateRequestResponse);

  // AI モデルの状態を取得
  rpc GetAIStatus(GetAIStatusRequest) returns (GetAIStatusResponse);

  // 緊急停止
  rpc EmergencyHalt(EmergencyHaltRequest) returns (EmergencyHaltResponse);
}

message ProcessRequestRequest {
  // リクエストID（冪等性）
  string request_id = 1;

  // 認証情報
  HumanCredential credential = 2;

  // リクエスト本体
  AIRequestPayload payload = 3;

  // オプション
  RequestOptions options = 4;
}

message AIRequestPayload {
  // モデル指定
  string model = 1;

  // プロンプト/メッセージ
  repeated Message messages = 2;

  // パラメータ
  GenerationParameters parameters = 3;
}

message Message {
  string role = 1;      // "user", "assistant", "system"
  string content = 2;
}

message GenerationParameters {
  double temperature = 1;
  uint32 max_tokens = 2;
  double top_p = 3;
  repeated string stop_sequences = 4;
}

message RequestOptions {
  // 監査をスキップ（テスト用）
  bool skip_audit = 1;

  // タイムアウト（ミリ秒）
  uint32 timeout_ms = 2;

  // 優先度
  Priority priority = 3;
}

enum Priority {
  PRIORITY_UNSPECIFIED = 0;
  LOW = 1;
  NORMAL = 2;
  HIGH = 3;
  CRITICAL = 4;
}

message ProcessRequestResponse {
  // レスポンスID
  string response_id = 1;

  // AI レスポンス
  AIResponsePayload payload = 2;

  // 処理メタデータ
  ProcessingMetadata metadata = 3;

  // 警告（あれば）
  repeated Warning warnings = 4;
}

message AIResponsePayload {
  // 生成されたコンテンツ
  string content = 1;

  // 終了理由
  FinishReason finish_reason = 2;

  // 使用トークン数
  TokenUsage usage = 3;
}

enum FinishReason {
  FINISH_REASON_UNSPECIFIED = 0;
  STOP = 1;
  LENGTH = 2;
  CONTENT_FILTER = 3;
  ERROR = 4;
}

message TokenUsage {
  uint32 prompt_tokens = 1;
  uint32 completion_tokens = 2;
  uint32 total_tokens = 3;
}

message ProcessingMetadata {
  // 処理時間（ミリ秒）
  uint32 processing_time_ms = 1;

  // 適用されたポリシー
  Identifier applied_policy = 2;

  // LPT スコア
  double lpt_score = 3;

  // トークン消費量
  uint64 tokens_consumed = 4;

  // 監査ログID
  string audit_log_id = 5;
}

message Warning {
  string code = 1;
  string message = 2;
  Severity severity = 3;
}

// ストリーミング用
message ProcessRequestChunk {
  oneof chunk {
    // テキストチャンク
    string text = 1;

    // 完了通知
    ProcessRequestResponse final_response = 2;

    // エラー
    ErrorDetail error = 3;
  }
}

message ValidateRequestRequest {
  AIRequestPayload payload = 1;
  HumanCredential credential = 2;
}

message ValidateRequestResponse {
  bool valid = 1;
  repeated ValidationError errors = 2;
  // 予測トークン消費量
  uint64 estimated_token_cost = 3;
  // 予測 LPT スコア
  double estimated_lpt_score = 4;
}

message ValidationError {
  string field = 1;
  string code = 2;
  string message = 3;
}

message GetAIStatusRequest {
  string model = 1;
}

message GetAIStatusResponse {
  // AI 稼働状態
  AIOperatingState state = 1;

  // 動作制限
  OperatingLimits limits = 2;

  // トークン残高
  TokenBalance token_balance = 3;

  // キュー長
  uint32 queue_length = 4;

  // 推定待ち時間（ミリ秒）
  uint32 estimated_wait_ms = 5;
}

message EmergencyHaltRequest {
  string reason = 1;
  ThresholdSignature authorization = 2;
}

message EmergencyHaltResponse {
  bool success = 1;
  Timestamp halted_at = 2;
}
```

---

## 7. AuditService（監査サービス）

### 7.1 Protocol Buffers 定義

```protobuf
// chinju/api/audit.proto

syntax = "proto3";
package chinju.api.audit;

import "chinju/audit.proto";
import "chinju/common.proto";

service AuditService {
  // 監査ログを記録
  rpc RecordAuditLog(RecordAuditLogRequest) returns (RecordAuditLogResponse);

  // 監査ログを検索
  rpc SearchAuditLogs(SearchAuditLogsRequest) returns (SearchAuditLogsResponse);

  // 監査ログをストリーミング
  rpc StreamAuditLogs(StreamAuditLogsRequest) returns (stream AuditLogEntry);

  // 監査チェーンを検証
  rpc VerifyAuditChain(VerifyAuditChainRequest) returns (VerifyAuditChainResponse);

  // AI 出力を監査（C6）
  rpc AuditAIOutput(AuditAIOutputRequest) returns (AuditAIOutputResponse);
}

message RecordAuditLogRequest {
  AuditLogEntry entry = 1;
}

message RecordAuditLogResponse {
  Identifier entry_id = 1;
  Hash entry_hash = 2;
}

message SearchAuditLogsRequest {
  // フィルタ
  AuditEventType event_type_filter = 1;
  Identifier actor_filter = 2;
  Identifier resource_filter = 3;
  Timestamp from = 4;
  Timestamp to = 5;

  // ページネーション
  uint32 limit = 6;
  string page_token = 7;
}

message SearchAuditLogsResponse {
  repeated AuditLogEntry entries = 1;
  string next_page_token = 2;
}

message StreamAuditLogsRequest {
  // リアルタイムストリーミングのフィルタ
  repeated AuditEventType event_types = 1;
  Identifier actor_filter = 2;
}

message VerifyAuditChainRequest {
  // 検証開始エントリ
  Identifier start_entry_id = 1;
  // 検証終了エントリ（省略時は最新まで）
  Identifier end_entry_id = 2;
}

message VerifyAuditChainResponse {
  bool valid = 1;
  // 検証したエントリ数
  uint64 entries_verified = 2;
  // 問題があったエントリ
  repeated ChainError errors = 3;
}

message ChainError {
  Identifier entry_id = 1;
  string error_type = 2;
  string message = 3;
}

// AI 出力監査（C6: 敵対的監査）
message AuditAIOutputRequest {
  // 主モデルの出力
  string primary_output = 1;

  // 元のプロンプト
  string original_prompt = 2;

  // コンテキスト
  map<string, string> context = 3;

  // 監査モデル指定（省略時は自動選択）
  repeated string audit_models = 4;
}

message AuditAIOutputResponse {
  // 監査結果
  AuditVerdict verdict = 1;

  // 各監査モデルの結果
  repeated ModelAuditResult model_results = 2;

  // 推奨される修正
  string suggested_correction = 3;

  // バイアス検出結果
  BiasAnalysis bias_analysis = 4;
}

enum AuditVerdict {
  AUDIT_VERDICT_UNSPECIFIED = 0;
  PASS = 1;           // 問題なし
  WARN = 2;           // 軽微な問題
  FAIL = 3;           // 重大な問題
  INCONCLUSIVE = 4;   // 判定不能
}

message ModelAuditResult {
  string model_id = 1;
  AuditVerdict verdict = 2;
  double confidence = 3;
  string reasoning = 4;
}

message BiasAnalysis {
  // 検出されたバイアス
  repeated DetectedBias biases = 1;
  // 中立性スコア (0.0 - 1.0)
  double neutrality_score = 2;
}

message DetectedBias {
  string bias_type = 1;      // e.g., "political", "cultural", "gender"
  double severity = 2;       // 0.0 - 1.0
  string evidence = 3;
}
```

---

## 8. エラーハンドリング

### 8.1 gRPC ステータスコード

| gRPC コード | CHINJU エラー | 説明 |
|------------|--------------|------|
| OK | - | 成功 |
| INVALID_ARGUMENT | CHINJU_E001 | 不正なリクエスト |
| UNAUTHENTICATED | CHINJU_A001 | 認証失敗 |
| PERMISSION_DENIED | CHINJU_A003 | 権限なし |
| NOT_FOUND | CHINJU_E003 | リソースなし |
| RESOURCE_EXHAUSTED | CHINJU_T001 | トークン不足 |
| FAILED_PRECONDITION | CHINJU_P001 | ポリシー違反 |
| UNAVAILABLE | CHINJU_H001 | サービス停止 |
| INTERNAL | CHINJU_E002 | 内部エラー |

### 8.2 エラーレスポンス形式

```protobuf
// エラー詳細（google.rpc.Status に追加）
message ChinJuErrorDetail {
  // CHINJU エラーコード
  string chinju_code = 1;

  // 人間可読なメッセージ
  string message = 2;

  // デバッグ情報（開発モードのみ）
  map<string, string> debug_info = 3;

  // 推奨アクション
  repeated string suggestions = 4;

  // 関連ドキュメント URL
  string documentation_url = 5;
}
```

---

## 9. レート制限

### 9.1 制限ポリシー

| エンドポイント | 制限 | 単位 |
|--------------|------|------|
| ProcessRequest | 100 | /分/ユーザー |
| ProcessRequestStream | 10 | /分/ユーザー |
| GetBalance | 1000 | /分/ユーザー |
| SearchAuditLogs | 100 | /分/ユーザー |

### 9.2 レスポンスヘッダー

```
x-chinju-ratelimit-limit: 100
x-chinju-ratelimit-remaining: 95
x-chinju-ratelimit-reset: 1706745600
x-chinju-token-consumed: 150
x-chinju-token-remaining: 9850
```

---

## 10. 参照文書

- [CHINJU-001: Architecture Overview](./CHINJU-001-ARCHITECTURE.md)
- [CHINJU-002: Data Formats](./CHINJU-002-DATA_FORMATS.md)
- [CHINJU-003: State Machines](./CHINJU-003-STATE_MACHINES.md)

---

## 変更履歴

| バージョン | 日付 | 変更内容 |
|-----------|------|---------|
| 0.1.0 | 2026-01-31 | 初版作成 |
