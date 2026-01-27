# QRNG (Quantum Random Number Generator) - Reference Design

## 概要

QRNGは、量子効果を利用して真正乱数を生成する。
C2（選択肢品質保証）のランダム射影、C4のナンス生成に使用。
AIによる乱数予測を物理的に不可能にする。

### 対応トレイト

```rust
pub trait SecureRandom: Send + Sync {
    fn generate(&self, len: usize) -> Result<Vec<u8>, Self::Error>;
    fn verify_entropy(&self) -> Result<f64, Self::Error>;
    fn pool_status(&self) -> Result<EntropyPoolStatus, Self::Error>;
    fn is_hardware_backed(&self) -> bool;
}
```

---

## 1. 乱数品質階層

```
┌─────────────────────────────────────────────────────┐
│  Level 4: QRNG（量子）                              │
│  • 量子光学効果（光子検出、真空揺らぎ）             │
│  • 理論的に予測不可能                              │
│  • NIST SP 800-90B Full Entropy                    │
├─────────────────────────────────────────────────────┤
│  Level 3: Hardware TRNG                             │
│  • 熱雑音、ジッタ、放射線                          │
│  • 物理的に予測困難                                │
│  • NIST SP 800-90B 準拠                            │
├─────────────────────────────────────────────────────┤
│  Level 2: Hardware TRNG + DRBG                      │
│  • TRNG → DRBG (Conditioning)                      │
│  • 統計的品質保証                                  │
├─────────────────────────────────────────────────────┤
│  Level 1: Software CSPRNG                           │
│  • ChaCha20, AES-CTR-DRBG                          │
│  • 初期シード依存                                  │
└─────────────────────────────────────────────────────┘
```

---

## 2. 設計パターン

### Pattern A: 専用QRNG（最高品質）

**用途**: 金融、国防、研究機関
**レベル**: Level 4
**コスト**: $2,000-$10,000

```
┌─────────────────────────────────────────────────────┐
│  Host Server                                        │
│  ┌───────────────────────────────────────────────┐  │
│  │  CHINJU Software Layer                        │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │ USB / PCIe              │
└──────────────────────────│──────────────────────────┘
                           │
┌──────────────────────────│──────────────────────────┐
│  QRNG Device             │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  Quantum Entropy Source                       │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Photon Detection                       │  │  │
│  │  │  • Single photon source                 │  │  │
│  │  │  • Beam splitter                        │  │  │
│  │  │  • Photon detector pair                 │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Post-Processing                        │  │  │
│  │  │  • Von Neumann extractor                │  │  │
│  │  │  • Toeplitz hashing                     │  │  │
│  │  │  • Health monitoring                    │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**製品例**:

| 製品 | ベンダー | 方式 | 速度 | 価格 |
|------|---------|------|------|------|
| Quantis PCIe | ID Quantique | 光子検出 | 240 Mbps | $3,000 |
| Quantis USB | ID Quantique | 光子検出 | 16 Mbps | $1,500 |
| qStream | QuintessenceLabs | 真空揺らぎ | 1 Gbps | $5,000+ |
| Quantum Origin | Quantinuum | Ion trap | API | SaaS |

**Rust実装例（ID Quantique）**:

```rust
use libidq::QuantisDevice;

pub struct QuantisQrng {
    device: QuantisDevice,
}

impl QuantisQrng {
    pub fn open(device_type: DeviceType, device_number: u32) -> Result<Self, QrngError> {
        let device = QuantisDevice::open(device_type, device_number)?;

        // 健全性チェック
        if !device.get_modules_status()? {
            return Err(QrngError::DeviceUnhealthy);
        }

        Ok(Self { device })
    }
}

impl SecureRandom for QuantisQrng {
    type Error = QrngError;

    fn generate(&self, len: usize) -> Result<Vec<u8>, Self::Error> {
        // 量子乱数を取得
        let mut buffer = vec![0u8; len];
        self.device.read(&mut buffer)?;
        Ok(buffer)
    }

    fn verify_entropy(&self) -> Result<f64, Self::Error> {
        // NIST SP 800-90B エントロピー推定
        let sample = self.generate(1024)?;
        let entropy = nist_sp800_90b::estimate_min_entropy(&sample);
        Ok(entropy)
    }

    fn pool_status(&self) -> Result<EntropyPoolStatus, Self::Error> {
        Ok(EntropyPoolStatus {
            available_bits: self.device.get_available_count()? * 8,
            last_reseed: Duration::ZERO, // 常に新鮮
            health: HealthStatus::Healthy,
        })
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

### Pattern B: Hardware TRNG（汎用）

**用途**: 企業、エッジデバイス
**レベル**: Level 2-3
**コスト**: $0-$100（チップ内蔵）

```
┌─────────────────────────────────────────────────────┐
│  MCU / SoC                                          │
│  ┌───────────────────────────────────────────────┐  │
│  │  CHINJU Software Layer                        │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  Hardware TRNG                                │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Noise Source                           │  │  │
│  │  │  • Ring oscillator jitter               │  │  │
│  │  │  • Thermal noise                        │  │  │
│  │  │  • Metastable flip-flop                 │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Conditioning                           │  │  │
│  │  │  • AES-CBC-MAC                          │  │  │
│  │  │  • SHA-256                              │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Health Tests                           │  │  │
│  │  │  • Repetition count                     │  │  │
│  │  │  • Adaptive proportion                  │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**プラットフォーム例**:

| プラットフォーム | 命令/レジスタ | 認証 |
|-----------------|--------------|------|
| Intel (RDRAND) | RDRAND, RDSEED | NIST CAVP |
| ARM (RNDR) | RNDR, RNDRRS | - |
| STM32 | RNG peripheral | NIST CAVP |
| NXP LPC | TRNG peripheral | - |

**Rust実装例（Intel RDRAND）**:

```rust
#[cfg(target_arch = "x86_64")]
pub struct RdrandTrng;

#[cfg(target_arch = "x86_64")]
impl SecureRandom for RdrandTrng {
    type Error = RngError;

    fn generate(&self, len: usize) -> Result<Vec<u8>, Self::Error> {
        use core::arch::x86_64::_rdrand64_step;

        let mut buffer = vec![0u8; len];
        let mut offset = 0;

        while offset < len {
            let mut val: u64 = 0;
            let success = unsafe { _rdrand64_step(&mut val) };

            if success == 0 {
                // RDRANDが失敗 → リトライまたはフォールバック
                return Err(RngError::HardwareFailure);
            }

            let bytes = val.to_le_bytes();
            let copy_len = (len - offset).min(8);
            buffer[offset..offset + copy_len].copy_from_slice(&bytes[..copy_len]);
            offset += copy_len;
        }

        Ok(buffer)
    }

    fn is_hardware_backed(&self) -> bool {
        // CPUID で RDRAND サポートを確認
        std::arch::x86_64::has_cpuid() &&
            (std::arch::x86_64::__cpuid(1).ecx & (1 << 30)) != 0
    }
}
```

---

### Pattern C: Secure Element TRNG

**用途**: IoT、組み込み
**レベル**: Level 2
**コスト**: $1-$5

```
┌─────────────────────────────────────────────────────┐
│  Host MCU                                           │
│  ┌───────────────────────────────────────────────┐  │
│  │  CHINJU Software Layer                        │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │ I2C                      │
└──────────────────────────│──────────────────────────┘
                           │
┌──────────────────────────│──────────────────────────┐
│  Secure Element          │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  ATECC608B / SE050                            │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  TRNG                                   │  │  │
│  │  │  • Hardware noise source                │  │  │
│  │  │  • AES-based DRBG                       │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Rust実装例（ATECC608B）**:

```rust
use cryptoauthlib::AtcaClient;

pub struct AteccTrng {
    client: AtcaClient,
}

impl SecureRandom for AteccTrng {
    type Error = AtcaError;

    fn generate(&self, len: usize) -> Result<Vec<u8>, Self::Error> {
        let mut buffer = vec![0u8; len];
        let mut offset = 0;

        while offset < len {
            // ATECC608Bは32バイト単位で乱数を生成
            let random = self.client.random()?;
            let copy_len = (len - offset).min(32);
            buffer[offset..offset + copy_len].copy_from_slice(&random[..copy_len]);
            offset += copy_len;
        }

        Ok(buffer)
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

## 3. ベンダー多様性

| レベル | プライマリ | セカンダリ | ターシャリ |
|--------|-----------|-----------|-----------|
| Level 4 | ID Quantique | QuintessenceLabs | Quantinuum |
| Level 3 | Intel RDRAND | ARM RNDR | STM32 RNG |
| Level 2 | ATECC608B | SE050 | OPTIGA Trust |

---

## 4. エントロピープール設計

### 複数ソースの混合

```rust
pub struct HybridEntropyPool {
    sources: Vec<Box<dyn SecureRandom>>,
    pool: [u8; 256],
    pool_bits: usize,
}

impl HybridEntropyPool {
    /// 複数ソースからエントロピーを収集して混合
    pub fn reseed(&mut self) -> Result<(), PoolError> {
        let mut collected = Vec::new();

        for source in &self.sources {
            if let Ok(entropy) = source.generate(32) {
                collected.extend(entropy);
            }
        }

        if collected.is_empty() {
            return Err(PoolError::NoEntropyAvailable);
        }

        // SHA-512でコンディショニング
        let hash = sha512::digest(&collected);
        self.pool.copy_from_slice(&hash[..256]);
        self.pool_bits = 256 * 8;

        Ok(())
    }
}

impl SecureRandom for HybridEntropyPool {
    type Error = PoolError;

    fn generate(&self, len: usize) -> Result<Vec<u8>, Self::Error> {
        if self.pool_bits < len * 8 {
            return Err(PoolError::InsufficientEntropy);
        }

        // プールから抽出してDRBGで拡張
        let mut drbg = ChaCha20Rng::from_seed(self.pool[..32].try_into()?);
        let mut buffer = vec![0u8; len];
        drbg.fill_bytes(&mut buffer);

        Ok(buffer)
    }
}
```

---

## 5. 健全性監視

### NIST SP 800-90B ヘルステスト

```rust
pub struct HealthMonitor {
    /// 連続一致カウント
    repetition_count: u32,
    /// 適応比率テストウィンドウ
    proportion_window: VecDeque<u8>,
    /// 故障フラグ
    failure_detected: bool,
}

impl HealthMonitor {
    pub fn check(&mut self, sample: u8) -> HealthStatus {
        // 1. Repetition Count Test
        if self.last_sample == sample {
            self.repetition_count += 1;
            if self.repetition_count > REPETITION_CUTOFF {
                self.failure_detected = true;
                return HealthStatus::Failed {
                    reason: "Repetition count exceeded".into(),
                };
            }
        } else {
            self.repetition_count = 1;
            self.last_sample = sample;
        }

        // 2. Adaptive Proportion Test
        self.proportion_window.push_back(sample);
        if self.proportion_window.len() > WINDOW_SIZE {
            self.proportion_window.pop_front();
        }

        let count = self.proportion_window.iter()
            .filter(|&&x| x == sample)
            .count();

        if count > PROPORTION_CUTOFF {
            self.failure_detected = true;
            return HealthStatus::Failed {
                reason: "Proportion test failed".into(),
            };
        }

        HealthStatus::Healthy
    }
}
```

---

## 参照文書

- `chinju/traits/hardware_abstraction.md` - トレイト定義
- `chinju/hardware/REQUIREMENTS.md` - 要件
- NIST SP 800-90A/B/C - Random Number Generation
- ID Quantique Quantis SDK Documentation
- Intel Digital Random Number Generator Guide
