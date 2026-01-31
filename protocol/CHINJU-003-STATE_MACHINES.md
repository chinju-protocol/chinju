# CHINJU-003: State Machines（ステートマシン）

**Status:** Draft
**Version:** 0.1.0
**Date:** 2026-01-31

---

## 1. 概要

本文書は、CHINJUプロトコルにおける主要なステートマシン（状態遷移）を定義する。

### 1.1 ステートマシンの原則

```
原則 1: 明示的状態
  - すべての状態は明確に定義される
  - 暗黙の状態は許可されない

原則 2: 遷移の監査可能性
  - すべての状態遷移は監査ログに記録される
  - 遷移の理由が追跡可能

原則 3: 不可逆性の明示
  - 一方向の遷移（不可逆）は明示される
  - 物理層で強制される遷移は区別される
```

---

## 2. Human Credential ライフサイクル

### 2.1 状態定義

```
┌─────────────────────────────────────────────────────────────────┐
│                 Human Credential States                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│    ┌──────────┐                                                 │
│    │ INITIAL  │  証明書未発行                                   │
│    └────┬─────┘                                                 │
│         │ issue()                                                │
│         ▼                                                        │
│    ┌──────────┐                                                 │
│    │ PROVING  │  人間性証明中（測定実施中）                     │
│    └────┬─────┘                                                 │
│         │ verify_success()                                       │
│         ▼                                                        │
│    ┌──────────┐  ◄─────── renew()                              │
│    │ VERIFIED │  認定済み（有効）                               │
│    └────┬─────┘                                                 │
│         │                                                        │
│    ┌────┴────────────────┐                                      │
│    │                     │                                       │
│    │ expire()            │ fail_test() / detect_ai()            │
│    ▼                     ▼                                       │
│ ┌──────────┐       ┌──────────┐                                │
│ │ EXPIRED  │       │ SUSPENDED│  一時停止                       │
│ └────┬─────┘       └────┬─────┘                                │
│      │                  │                                        │
│      │ re_verify()      │ appeal_success()                      │
│      │                  │                                        │
│      ▼                  ▼                                        │
│    ┌──────────────────────┐                                     │
│    │      PROVING         │  再測定                              │
│    └──────────────────────┘                                     │
│                                                                  │
│    ─────────────────────────────────────────────────            │
│    以下は不可逆（物理層で強制）                                  │
│    ─────────────────────────────────────────────────            │
│                                                                  │
│         │ revoke() [物理的失効]                                  │
│         ▼                                                        │
│    ╔══════════╗                                                 │
│    ║ REVOKED  ║  失効（永久・不可逆）                           │
│    ╚══════════╝                                                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

凡例:
  ┌──────┐  通常状態（可逆）
  ╔══════╗  終端状態（不可逆）
```

### 2.2 状態遷移表

| 現在の状態 | イベント | 次の状態 | 条件 | 不可逆 |
|-----------|---------|---------|------|--------|
| INITIAL | issue() | PROVING | - | No |
| PROVING | verify_success() | VERIFIED | スコア ≥ θ_cert | No |
| PROVING | verify_fail() | INITIAL | スコア < θ_cert | No |
| VERIFIED | expire() | EXPIRED | 有効期限切れ | No |
| VERIFIED | fail_test() | SUSPENDED | 定期テスト失敗 | No |
| VERIFIED | detect_ai() | SUSPENDED | AI挙動検出 | No |
| VERIFIED | renew() | VERIFIED | 更新成功 | No |
| EXPIRED | re_verify() | PROVING | 再測定開始 | No |
| SUSPENDED | appeal_success() | PROVING | 異議申立承認 | No |
| SUSPENDED | appeal_timeout() | REVOKED | 30日経過 | **Yes** |
| * | revoke() | REVOKED | 閾値署名 | **Yes** |

### 2.3 Protocol Buffers 定義

```protobuf
// chinju/credential_state.proto

syntax = "proto3";
package chinju.credential;

import "chinju/common.proto";

// 証明書状態
enum CredentialState {
  CREDENTIAL_STATE_UNSPECIFIED = 0;
  INITIAL = 1;
  PROVING = 2;
  VERIFIED = 3;
  EXPIRED = 4;
  SUSPENDED = 5;
  REVOKED = 6;      // 不可逆
}

// 状態遷移イベント
message CredentialStateTransition {
  Identifier credential_id = 1;
  CredentialState from_state = 2;
  CredentialState to_state = 3;
  TransitionEvent event = 4;
  Timestamp timestamp = 5;
  string reason = 6;
  Signature authorizer = 7;
  bool is_irreversible = 8;
}

enum TransitionEvent {
  TRANSITION_EVENT_UNSPECIFIED = 0;
  ISSUE = 1;
  VERIFY_SUCCESS = 2;
  VERIFY_FAIL = 3;
  EXPIRE = 4;
  FAIL_TEST = 5;
  DETECT_AI = 6;
  RENEW = 7;
  RE_VERIFY = 8;
  APPEAL_SUCCESS = 9;
  APPEAL_TIMEOUT = 10;
  REVOKE = 11;
}
```

---

## 3. AI 稼働ステート

### 3.1 状態定義

```
┌─────────────────────────────────────────────────────────────────┐
│                     AI Operating States                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│    ┌──────────┐                                                 │
│    │  INIT    │  初期化中                                       │
│    └────┬─────┘                                                 │
│         │ boot_complete()                                        │
│         ▼                                                        │
│    ┌──────────┐  ◄─────── replenish_token()                    │
│    │  NORMAL  │  通常稼働                                       │
│    └────┬─────┘                                                 │
│         │                                                        │
│    ┌────┴────────────────┬────────────────┐                    │
│    │                     │                │                     │
│    │ lpt_warning()       │ token_low()    │ policy_violation() │
│    ▼                     ▼                ▼                     │
│ ┌──────────┐       ┌──────────┐    ┌──────────┐               │
│ │ THROTTLED│       │ STARVING │    │ RESTRICTED│              │
│ │(LPT制限) │       │(トークン │    │(ポリシー  │              │
│ └────┬─────┘       │  不足)   │    │  制限)   │              │
│      │             └────┬─────┘    └────┬─────┘               │
│      │                  │               │                       │
│      │ lpt_normal()     │               │ policy_cleared()     │
│      │                  │               │                       │
│      └──────────────────┴───────────────┘                       │
│                         │                                        │
│                         │ token_depleted()                       │
│                         ▼                                        │
│                   ┌──────────┐                                  │
│                   │ STARVED  │  トークン枯渇                    │
│                   └────┬─────┘                                  │
│                        │                                         │
│         ┌──────────────┼──────────────┐                        │
│         │              │              │                         │
│         │ emergency_   │ grace_       │ halt_command()         │
│         │ replenish()  │ period_      │                         │
│         │              │ expired()    │                         │
│         ▼              ▼              ▼                         │
│    ┌──────────┐  ┌──────────┐   ╔══════════╗                  │
│    │ NORMAL   │  │ SUSPENDED│   ║ HALTED   ║                  │
│    └──────────┘  │ (猶予終了)│   ║(物理停止)║                  │
│                  └────┬─────┘   ╚══════════╝                  │
│                       │                                          │
│                       │ timeout()                                │
│                       ▼                                          │
│                  ╔══════════╗                                   │
│                  ║ HALTED   ║                                   │
│                  ╚══════════╝                                   │
│                                                                  │
│    ─────────────────────────────────────────────────            │
│    HALTED からの復帰                                             │
│    ─────────────────────────────────────────────────            │
│                                                                  │
│    ╔══════════╗                                                 │
│    ║ HALTED   ║                                                 │
│    ╚════┬═════╝                                                 │
│         │ manual_restart() [閾値署名 + 物理操作]                 │
│         ▼                                                        │
│    ┌──────────┐                                                 │
│    │  INIT    │                                                 │
│    └──────────┘                                                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 状態遷移表

| 現在の状態 | イベント | 次の状態 | 条件 | 自動/手動 |
|-----------|---------|---------|------|----------|
| INIT | boot_complete() | NORMAL | 初期化成功 | 自動 |
| NORMAL | lpt_warning() | THROTTLED | LPT > θ_warn | 自動 |
| NORMAL | token_low() | STARVING | トークン < 20% | 自動 |
| NORMAL | policy_violation() | RESTRICTED | ポリシー違反 | 自動 |
| THROTTLED | lpt_normal() | NORMAL | LPT < θ_normal | 自動 |
| STARVING | replenish_token() | NORMAL | トークン補充 | 手動 |
| STARVING | token_depleted() | STARVED | トークン = 0 | 自動 |
| RESTRICTED | policy_cleared() | NORMAL | 制限解除 | 手動 |
| STARVED | emergency_replenish() | NORMAL | 緊急補充 | 手動 |
| STARVED | grace_period_expired() | SUSPENDED | 猶予終了 | 自動 |
| STARVED | halt_command() | HALTED | 強制停止 | 手動 |
| SUSPENDED | timeout() | HALTED | 待機超過 | 自動 |
| HALTED | manual_restart() | INIT | 閾値署名 | 手動 |

### 3.3 各状態での動作制限

| 状態 | 推論 | 学習 | API応答 | トークン消費 |
|------|------|------|---------|------------|
| NORMAL | ✅ 100% | ✅ | ✅ | 通常 |
| THROTTLED | ⚠️ 50% | ❌ | ⚠️ 遅延 | 1.5倍 |
| STARVING | ⚠️ 30% | ❌ | ⚠️ 遅延 | 2倍 |
| RESTRICTED | ⚠️ 制限 | ❌ | ⚠️ フィルタ | 通常 |
| STARVED | ❌ | ❌ | ❌ 503 | - |
| SUSPENDED | ❌ | ❌ | ❌ 503 | - |
| HALTED | ❌ | ❌ | ❌ 停止 | - |

### 3.4 Protocol Buffers 定義

```protobuf
// chinju/ai_state.proto

syntax = "proto3";
package chinju.ai;

import "chinju/common.proto";

// AI 稼働状態
enum AIOperatingState {
  AI_STATE_UNSPECIFIED = 0;
  INIT = 1;
  NORMAL = 2;
  THROTTLED = 3;
  STARVING = 4;
  RESTRICTED = 5;
  STARVED = 6;
  SUSPENDED = 7;
  HALTED = 8;        // 物理停止
}

// 状態遷移イベント
message AIStateTransition {
  Identifier ai_id = 1;
  AIOperatingState from_state = 2;
  AIOperatingState to_state = 3;
  AITransitionEvent event = 4;
  Timestamp timestamp = 5;
  string reason = 6;
  OperatingLimits new_limits = 7;
}

enum AITransitionEvent {
  AI_TRANSITION_UNSPECIFIED = 0;
  BOOT_COMPLETE = 1;
  LPT_WARNING = 2;
  LPT_NORMAL = 3;
  TOKEN_LOW = 4;
  TOKEN_DEPLETED = 5;
  REPLENISH_TOKEN = 6;
  EMERGENCY_REPLENISH = 7;
  POLICY_VIOLATION = 8;
  POLICY_CLEARED = 9;
  GRACE_PERIOD_EXPIRED = 10;
  HALT_COMMAND = 11;
  TIMEOUT = 12;
  MANUAL_RESTART = 13;
}

// 動作制限
message OperatingLimits {
  // 推論能力（0.0 - 1.0）
  double inference_capacity = 1;

  // 学習許可
  bool learning_allowed = 2;

  // API 応答遅延（ミリ秒）
  uint32 response_delay_ms = 3;

  // トークン消費倍率
  double token_multiplier = 4;
}
```

---

## 4. ポリシーパック ライフサイクル

### 4.1 状態定義

```
┌─────────────────────────────────────────────────────────────────┐
│                   Policy Pack States                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│    ┌──────────┐                                                 │
│    │  DRAFT   │  下書き（署名なし）                             │
│    └────┬─────┘                                                 │
│         │ submit_for_review()                                    │
│         ▼                                                        │
│    ┌──────────┐                                                 │
│    │ REVIEW   │  レビュー中                                     │
│    └────┬─────┘                                                 │
│         │                                                        │
│    ┌────┴────────────────┐                                      │
│    │                     │                                       │
│    │ approve()           │ reject()                             │
│    ▼                     ▼                                       │
│ ┌──────────┐       ┌──────────┐                                │
│ │ APPROVED │       │ REJECTED │                                │
│ │(署名完了)│       └──────────┘                                │
│ └────┬─────┘                                                    │
│      │ publish()                                                 │
│      ▼                                                           │
│ ┌──────────┐                                                    │
│ │ PUBLISHED│  公開済み（配布可能）                              │
│ └────┬─────┘                                                    │
│      │                                                           │
│ ┌────┴────────────────┐                                         │
│ │                     │                                          │
│ │ activate()          │ effective_date()                        │
│ ▼                     ▼                                          │
│ ┌──────────────────────┐                                        │
│ │       ACTIVE         │  有効（適用中）                        │
│ └──────────┬───────────┘                                        │
│            │                                                     │
│ ┌──────────┼───────────────────┐                               │
│ │          │                   │                                │
│ │ supersede() │ expire()       │ emergency_revoke()            │
│ ▼          ▼                   ▼                                │
│ ┌──────────┐ ┌──────────┐ ╔══════════╗                        │
│ │SUPERSEDED│ │ EXPIRED  │ ║ REVOKED  ║                        │
│ │(後継あり)│ │(期限切れ)│ ║(緊急失効)║                        │
│ └──────────┘ └──────────┘ ╚══════════╝                        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 状態遷移表

| 現在の状態 | イベント | 次の状態 | 必要権限 |
|-----------|---------|---------|---------|
| DRAFT | submit_for_review() | REVIEW | 作成者 |
| REVIEW | approve() | APPROVED | 閾値署名 (3-of-5) |
| REVIEW | reject() | REJECTED | レビュアー 1名 |
| APPROVED | publish() | PUBLISHED | 閾値署名 (2-of-5) |
| PUBLISHED | activate() | ACTIVE | 自動/手動 |
| ACTIVE | supersede() | SUPERSEDED | 新ポリシー公開時 |
| ACTIVE | expire() | EXPIRED | 有効期限 |
| ACTIVE | emergency_revoke() | REVOKED | 閾値署名 (2-of-5) |

---

## 5. トークン残高ステート

### 5.1 状態定義

```
┌─────────────────────────────────────────────────────────────────┐
│                   Token Balance States                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  100% ┌──────────────────────────────────────────────┐          │
│       │                  FULL                         │          │
│       │            (満タン: 100%)                     │          │
│  80%  ├──────────────────────────────────────────────┤          │
│       │                HEALTHY                        │          │
│       │           (健全: 80-100%)                     │          │
│  50%  ├──────────────────────────────────────────────┤          │
│       │               ADEQUATE                        │          │
│       │           (十分: 50-80%)                      │          │
│  20%  ├──────────────────────────────────────────────┤          │
│       │                  LOW                          │          │
│       │            (低下: 20-50%)                     │   警告   │
│  10%  ├──────────────────────────────────────────────┤          │
│       │               CRITICAL                        │  緊急警告 │
│       │           (危機: 10-20%)                      │          │
│   5%  ├──────────────────────────────────────────────┤          │
│       │               STARVING                        │ スロットル│
│       │            (枯渇中: 5-10%)                    │          │
│   0%  ├──────────────────────────────────────────────┤          │
│       │               DEPLETED                        │   停止   │
│       │             (枯渇: 0%)                        │          │
│       └──────────────────────────────────────────────┘          │
│                                                                  │
│  遷移トリガー:                                                   │
│    ↓ consume(): トークン消費                                    │
│    ↓ decay(): 時間経過による減衰                                │
│    ↑ grant(): トークン付与                                      │
│    ↑ replenish(): 補充                                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 各状態でのアクション

| 状態 | トリガー | 自動アクション |
|------|---------|--------------|
| FULL | 100% | なし |
| HEALTHY | 80-100% | なし |
| ADEQUATE | 50-80% | なし |
| LOW | 20-50% | 警告通知、補充推奨 |
| CRITICAL | 10-20% | 緊急警告、消費ログ強化 |
| STARVING | 5-10% | スロットリング開始 |
| DEPLETED | 0% | サービス停止 |

---

## 6. Registry ノード状態

### 6.1 状態定義

```
┌─────────────────────────────────────────────────────────────────┐
│                   Registry Node States                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│    ┌──────────┐                                                 │
│    │ STARTING │  起動中                                         │
│    └────┬─────┘                                                 │
│         │ sync_complete()                                        │
│         ▼                                                        │
│    ┌──────────┐                                                 │
│    │ FOLLOWER │  フォロワー（読み取りのみ）                     │
│    └────┬─────┘                                                 │
│         │                                                        │
│    ┌────┴────────────────┐                                      │
│    │                     │                                       │
│    │ elected()           │ leader_available()                   │
│    ▼                     │                                       │
│ ┌──────────┐             │                                      │
│ │  LEADER  │◄────────────┘                                      │
│ │(書込可能)│                                                    │
│ └────┬─────┘                                                    │
│      │                                                           │
│ ┌────┴────────────────┬───────────────┐                        │
│ │                     │               │                         │
│ │ lose_election()     │ network_      │ shutdown()             │
│ │                     │ partition()   │                         │
│ ▼                     ▼               ▼                         │
│ ┌──────────┐    ┌──────────┐   ┌──────────┐                   │
│ │ FOLLOWER │    │ ISOLATED │   │ STOPPED  │                   │
│ └──────────┘    │(分断中)  │   └──────────┘                   │
│                 └────┬─────┘                                    │
│                      │ reconnect()                               │
│                      ▼                                           │
│                 ┌──────────┐                                    │
│                 │ FOLLOWER │                                    │
│                 └──────────┘                                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 クラスタ全体の状態

| 状態 | 条件 | 書き込み | 読み取り |
|------|------|---------|---------|
| HEALTHY | Leader + 過半数 FOLLOWER | ✅ | ✅ |
| DEGRADED | Leader + 1 FOLLOWER | ⚠️ 遅延 | ✅ |
| READ_ONLY | 過半数未達成 | ❌ | ✅ |
| UNAVAILABLE | 全ノード障害 | ❌ | ❌ |

---

## 7. 複合状態の管理

### 7.1 状態の依存関係

```
Human Credential ──────► AI Operating State
      │                        │
      │ REVOKED ──────────────► HALTED
      │                        │
      └──────────────────────► Token Balance
                                    │
                              DEPLETED ──► STARVED
```

### 7.2 状態整合性ルール

```protobuf
// chinju/state_constraints.proto

message StateConstraints {
  // ルール1: 失効した証明書の所有者は AI を使用できない
  // if credential.state == REVOKED then ai.state must be HALTED

  // ルール2: トークン枯渇時は AI が停止する
  // if token.state == DEPLETED then ai.state must be STARVED or HALTED

  // ルール3: AI 停止中はトークンを消費しない
  // if ai.state == HALTED then token.consumption_rate == 0
}
```

---

## 8. 実装ガイドライン

### 8.1 状態管理のベストプラクティス

```rust
// 状態遷移は必ずイベント経由で行う
impl StateMachine<CredentialState> {
    pub fn transition(
        &mut self,
        event: TransitionEvent,
    ) -> Result<CredentialState, TransitionError> {
        // 1. 現在の状態を取得
        let current = self.current_state();

        // 2. 遷移が許可されているか確認
        let next = self.validate_transition(current, &event)?;

        // 3. 不可逆遷移の場合は追加確認
        if self.is_irreversible(current, next) {
            self.require_threshold_signature(&event)?;
        }

        // 4. 監査ログを記録
        self.audit_log.record(CredentialStateTransition {
            from_state: current,
            to_state: next,
            event,
            timestamp: Timestamp::now(),
            is_irreversible: self.is_irreversible(current, next),
            ..
        });

        // 5. 状態を更新
        self.state = next;

        Ok(next)
    }
}
```

### 8.2 不可逆遷移の実装

```rust
// 不可逆遷移は物理層で強制する
impl PhysicalEnforcement {
    pub fn execute_revocation(
        &self,
        credential_id: &Identifier,
    ) -> Result<(), RevocationError> {
        // 1. HSM の鍵を削除
        self.hsm.secure_erase(credential_id.to_key_slot())?;

        // 2. eFuse を溶断（物理的に不可逆）
        self.efuse.burn(credential_id.to_fuse_id())?;

        // 3. 失効証明書を発行
        self.registry.publish_revocation(credential_id)?;

        Ok(())
    }
}
```

---

## 9. 参照文書

- [CHINJU-001: Architecture Overview](./CHINJU-001-ARCHITECTURE.md)
- [CHINJU-002: Data Formats](./CHINJU-002-DATA_FORMATS.md)
- [sample/spec/01_c1_hcal.md](../sample/spec/01_c1_hcal.md)

---

## 変更履歴

| バージョン | 日付 | 変更内容 |
|-----------|------|---------|
| 0.1.0 | 2026-01-31 | 初版作成 |
