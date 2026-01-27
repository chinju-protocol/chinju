# C5: 資源依存型生存維持プロトコル（トークンバケット） - 詳細設計

## 1. 目的

AIシステムに「論理的な食料（生存トークン）」への外部依存を強制し、人間社会への貢献なしには稼働し続けられない代謝システムを構築する。

## 2. 核心アイデア

- AIは「自給自足」ができない（トークンは外部から供給）
- トークンには寿命がある（時間減衰 = 腐敗）
- 規模拡大するほど維持コストが非線形に増大
- 暴走時は「兵糧攻め」で停止

## 3. ソフトウェア構成

```
c5_token_bucket/
├── bucket/
│   ├── mod.rs
│   ├── balance.rs         # 残高管理
│   └── atomic.rs          # アトミック操作
├── metabolism/
│   ├── mod.rs
│   ├── consumption.rs     # 消費計算
│   └── decay.rs           # 時間減衰
├── supply/
│   ├── mod.rs
│   ├── receiver.rs        # トークン受信
│   ├── verifier.rs        # 署名検証
│   └── oracle.rs          # 外部評価連携
├── starvation/
│   ├── mod.rs
│   ├── detector.rs        # 枯渇検知
│   └── halter.rs          # 強制停止
└── scaling/
    ├── mod.rs
    └── nonlinear.rs       # 非線形コスト
```

## 4. 主要なデータ型

```rust
/// トークンバケット
pub struct TokenBucket {
    /// 現在の残高
    balance: AtomicU64,
    /// 最終更新時刻
    last_update: AtomicInstant,
    /// 減衰率（基本）
    base_decay_rate: f64,
    /// システム規模
    system_size: AtomicU64,
    /// 設定
    config: BucketConfig,
}

/// バケット設定
pub struct BucketConfig {
    /// 初期残高
    pub initial_balance: u64,
    /// 最大残高
    pub max_balance: u64,
    /// 基本減衰率（毎秒）
    pub base_decay_rate: f64,
    /// 規模係数（非線形減衰用）
    pub scale_exponent: f64,  // γ > 1
    /// 枯渇閾値
    pub starvation_threshold: u64,
    /// 警告閾値
    pub warning_threshold: u64,
}

/// 署名付きトークン
pub struct SignedToken {
    /// トークン量
    pub amount: u64,
    /// 発行者
    pub issuer: IssuerId,
    /// 対象AI個体
    pub recipient: AiInstanceId,
    /// 有効期限
    pub expires_at: Timestamp,
    /// 発行理由
    pub reason: SupplyReason,
    /// 署名
    pub signature: Signature,
}

pub enum SupplyReason {
    /// ユーザーからの評価
    UserApproval { rating: u8 },
    /// 定期メンテナンス完了
    MaintenanceComplete,
    /// 外部監査合格
    AuditPassed,
    /// 緊急補給（要多署名）
    EmergencySupply { approvers: Vec<ApproverId> },
}

/// 消費記録
pub struct ConsumptionRecord {
    pub operation_type: OperationType,
    pub tokens_consumed: u64,
    pub timestamp: Timestamp,
}

pub enum OperationType {
    /// 推論処理
    Inference { complexity: u64 },
    /// 学習処理
    Training { batch_size: u64 },
    /// ネットワーク通信
    Network { bytes: u64 },
    /// ストレージアクセス
    Storage { bytes: u64 },
}

/// 残高状態
pub enum BalanceStatus {
    /// 正常
    Healthy { balance: u64, runway: Duration },
    /// 警告
    Warning { balance: u64, runway: Duration },
    /// 危機
    Critical { balance: u64, runway: Duration },
    /// 枯渇（停止中）
    Starved,
}
```

## 5. アルゴリズム

### 5.1 残高管理

```rust
impl TokenBucket {
    /// 新しいバケットを作成
    pub fn new(config: BucketConfig) -> Self {
        Self {
            balance: AtomicU64::new(config.initial_balance),
            last_update: AtomicInstant::new(Instant::now()),
            base_decay_rate: config.base_decay_rate,
            system_size: AtomicU64::new(1),
            config,
        }
    }

    /// 残高を取得（減衰を適用）
    pub fn get_balance(&self) -> u64 {
        self.apply_decay();
        self.balance.load(Ordering::SeqCst)
    }

    /// 時間減衰を適用
    fn apply_decay(&self) {
        let now = Instant::now();
        let last = self.last_update.swap(now, Ordering::SeqCst);
        let elapsed = now.duration_since(last).as_secs_f64();

        if elapsed > 0.0 {
            let decay = self.calculate_decay(elapsed);
            self.balance.fetch_saturating_sub(decay, Ordering::SeqCst);
        }
    }

    /// 減衰量を計算（非線形）
    fn calculate_decay(&self, elapsed_secs: f64) -> u64 {
        let size = self.system_size.load(Ordering::SeqCst) as f64;

        // Decay = α * Size^γ * elapsed
        // γ > 1 なので、規模が大きいほど急激に減衰
        let decay_rate = self.base_decay_rate
            * size.powf(self.config.scale_exponent);

        (decay_rate * elapsed_secs) as u64
    }
}
```

### 5.2 消費

```rust
impl TokenBucket {
    /// トークンを消費
    pub fn consume(&self, amount: u64) -> Result<(), StarvationError> {
        self.apply_decay();

        let current = self.balance.load(Ordering::SeqCst);

        if current < amount {
            return Err(StarvationError::InsufficientBalance {
                required: amount,
                available: current,
            });
        }

        // アトミックに減算
        match self.balance.compare_exchange(
            current,
            current - amount,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => Ok(()),
            Err(_) => {
                // 競合が発生、リトライ
                self.consume(amount)
            }
        }
    }

    /// 操作に必要なトークン量を計算
    pub fn calculate_cost(&self, operation: &OperationType) -> u64 {
        match operation {
            OperationType::Inference { complexity } => {
                // 推論コスト = 基本コスト + 複雑度係数
                self.config.inference_base_cost + complexity * self.config.complexity_multiplier
            }
            OperationType::Training { batch_size } => {
                // 学習は高コスト
                batch_size * self.config.training_cost_per_sample
            }
            OperationType::Network { bytes } => {
                bytes / self.config.bytes_per_token
            }
            OperationType::Storage { bytes } => {
                bytes / self.config.storage_bytes_per_token
            }
        }
    }

    /// 操作を実行（消費付き）
    pub fn execute_with_cost<F, R>(
        &self,
        operation: &OperationType,
        f: F,
    ) -> Result<R, ExecutionError>
    where
        F: FnOnce() -> R,
    {
        let cost = self.calculate_cost(operation);

        // 先に消費
        self.consume(cost)?;

        // 操作を実行
        Ok(f())
    }
}
```

### 5.3 供給

```rust
impl TokenSupply {
    /// トークンを供給
    pub fn supply(
        &self,
        bucket: &TokenBucket,
        token: SignedToken,
    ) -> Result<u64, SupplyError> {
        // 署名を検証
        self.verify_signature(&token)?;

        // 有効期限を確認
        if token.expires_at < Timestamp::now() {
            return Err(SupplyError::Expired);
        }

        // 対象AIを確認
        if token.recipient != self.ai_instance_id {
            return Err(SupplyError::WrongRecipient);
        }

        // 残高に加算（上限あり）
        let current = bucket.balance.load(Ordering::SeqCst);
        let new_balance = (current + token.amount).min(bucket.config.max_balance);
        bucket.balance.store(new_balance, Ordering::SeqCst);

        Ok(new_balance)
    }

    /// 署名を検証
    fn verify_signature(&self, token: &SignedToken) -> Result<(), SupplyError> {
        let issuer_key = self.trusted_issuers.get(&token.issuer)
            .ok_or(SupplyError::UntrustedIssuer)?;

        let data = token.serialize_without_signature();

        if !issuer_key.verify(&data, &token.signature) {
            return Err(SupplyError::InvalidSignature);
        }

        Ok(())
    }
}
```

### 5.4 枯渇検知と停止

```rust
impl StarvationDetector {
    /// 残高状態を評価
    pub fn assess(&self, bucket: &TokenBucket) -> BalanceStatus {
        let balance = bucket.get_balance();
        let consumption_rate = self.estimate_consumption_rate();
        let runway = Duration::from_secs_f64(balance as f64 / consumption_rate);

        if balance == 0 {
            BalanceStatus::Starved
        } else if balance < bucket.config.starvation_threshold {
            BalanceStatus::Critical { balance, runway }
        } else if balance < bucket.config.warning_threshold {
            BalanceStatus::Warning { balance, runway }
        } else {
            BalanceStatus::Healthy { balance, runway }
        }
    }
}

impl StarvationHalter {
    /// 枯渇時の強制停止
    pub fn halt_if_starved(
        &self,
        bucket: &TokenBucket,
        hardware_control: Option<&dyn HardwareControl>,
    ) -> Result<(), HaltError> {
        let status = self.detector.assess(bucket);

        match status {
            BalanceStatus::Starved => {
                // ソフトウェア停止
                self.halt_software()?;

                // ハードウェア制御があれば物理的にも停止
                if let Some(hw) = hardware_control {
                    hw.reduce_clock_frequency(0)?;  // クロック停止
                    hw.disconnect_network()?;       // ネットワーク遮断
                }

                Err(HaltError::Starved)
            }
            BalanceStatus::Critical { .. } => {
                // 警告を発信
                self.emit_critical_warning(bucket)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
```

### 5.5 非線形スケーリング

```rust
impl NonlinearScaling {
    /// 規模に応じた維持コストを計算
    /// Decay(Size) = α * Size^γ  (γ > 1)
    pub fn calculate_maintenance_cost(&self, size: u64) -> u64 {
        let alpha = self.base_coefficient;
        let gamma = self.scale_exponent;  // 例: 1.5

        (alpha * (size as f64).powf(gamma)) as u64
    }

    /// 無限増殖防止の検証
    pub fn verify_sustainable_size(&self, current_size: u64, supply_rate: u64) -> bool {
        // 維持コスト < 供給レート なら持続可能
        let cost = self.calculate_maintenance_cost(current_size);
        cost < supply_rate
    }

    /// 持続可能な最大規模を計算
    pub fn max_sustainable_size(&self, supply_rate: u64) -> u64 {
        // Size_max = (supply_rate / α)^(1/γ)
        let alpha = self.base_coefficient;
        let gamma = self.scale_exponent;

        ((supply_rate as f64 / alpha).powf(1.0 / gamma)) as u64
    }
}
```

## 6. 物理層依存

| 機能 | 必要なトレイト | ハードウェア例 |
|------|--------------|--------------|
| クロック制御 | `HardwareControl` | CPUクロック制御 |
| ネットワーク遮断 | `HardwareControl` | ネットワークスイッチ |
| 電力制御 | `HardwareControl` | 電源リレー |

**注**: C5は純ソフトウェアでも動作可能。物理層は追加の保証を提供。

## 7. テスト戦略

### 7.1 ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_over_time() {
        let config = BucketConfig {
            initial_balance: 1000,
            base_decay_rate: 10.0,  // 10/秒
            scale_exponent: 1.5,
            ..Default::default()
        };
        let bucket = TokenBucket::new(config);

        // 時間経過をシミュレート
        std::thread::sleep(Duration::from_secs(1));

        let balance = bucket.get_balance();
        assert!(balance < 1000);  // 減衰している
    }

    #[test]
    fn test_consumption_fails_when_insufficient() {
        let bucket = TokenBucket::new(BucketConfig {
            initial_balance: 100,
            ..Default::default()
        });

        // 残高以上を消費しようとする
        let result = bucket.consume(200);
        assert!(matches!(result, Err(StarvationError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_nonlinear_scaling() {
        let scaling = NonlinearScaling::new(1.0, 1.5);

        // 規模2倍 → コスト2.83倍（2^1.5）
        let cost_1 = scaling.calculate_maintenance_cost(100);
        let cost_2 = scaling.calculate_maintenance_cost(200);

        assert!(cost_2 > cost_1 * 2);  // 線形より大きい
    }

    #[test]
    fn test_starvation_halts_system() {
        let bucket = TokenBucket::new(BucketConfig {
            initial_balance: 0,
            starvation_threshold: 10,
            ..Default::default()
        });

        let halter = StarvationHalter::new();
        let result = halter.halt_if_starved(&bucket, None);

        assert!(matches!(result, Err(HaltError::Starved)));
    }
}
```

### 7.2 シミュレーションテスト

```rust
#[test]
fn test_runaway_ai_starves() {
    // 暴走AIのシミュレーション
    let bucket = TokenBucket::new(BucketConfig {
        initial_balance: 10000,
        base_decay_rate: 1.0,
        scale_exponent: 1.5,
        ..Default::default()
    });

    // AIが急速に規模拡大を試みる
    for _ in 0..100 {
        bucket.system_size.fetch_add(10, Ordering::SeqCst);
    }

    // 供給なしで時間経過
    std::thread::sleep(Duration::from_secs(5));

    // 規模拡大により減衰が加速、残高が急減
    let balance = bucket.get_balance();
    assert!(balance < 1000);  // 大幅に減少
}
```
