# Tamper Detection - Reference Design

## 概要

タンパー検出は、物理的な攻撃からハードウェアを保護する。
検出時に秘密情報を消去し、攻撃者が情報を取得することを防ぐ。

---

## 1. 脅威モデル

### 攻撃カテゴリ

| カテゴリ | 攻撃例 | 対策レベル |
|---------|--------|-----------|
| **非侵襲的** | 電力解析、電磁解析、タイミング | Level 2+ |
| **半侵襲的** | 光学プローブ、レーザー故障注入 | Level 3+ |
| **侵襲的** | デキャップ、マイクロプロービング | Level 4 |

### FIPS 140-2/3 対応

| Level | 要件 |
|-------|------|
| Level 1 | 製品グレードの筐体 |
| Level 2 | タンパー証跡（封印シール等） |
| Level 3 | タンパー検出・応答（ゼロ化） |
| Level 4 | 環境故障保護（EFP/EFT） |

---

## 2. 設計パターン

### Pattern A: 電気的検出（基本）

**用途**: 企業向け、IoT
**レベル**: Level 2-3
**コスト**: $10-$50

```
┌─────────────────────────────────────────────────────┐
│  Tamper Detection Circuit                           │
│  ┌───────────────────────────────────────────────┐  │
│  │  Voltage Monitor                              │  │
│  │  ┌───────────┐                               │  │
│  │  │ Vcc ─┬───│ Comparator │─── Alert          │  │
│  │  │      │   └───────────┘                    │  │
│  │  │    ┌─┴─┐                                  │  │
│  │  │    │ R │  Reference                       │  │
│  │  │    └─┬─┘                                  │  │
│  │  │      │                                    │  │
│  │  │     GND                                   │  │
│  │  └───────────────────────────────────────────┘  │
│  │                                               │  │
│  │  Temperature Monitor                          │  │
│  │  ┌───────────┐                               │  │
│  │  │ Thermistor│─── ADC ─── Comparator ─ Alert │  │
│  │  └───────────┘                               │  │
│  │                                               │  │
│  │  Case Open Switch                             │  │
│  │  ┌───────────┐                               │  │
│  │  │ μ-switch  │─── GPIO ─── Alert             │  │
│  │  └───────────┘                               │  │
│  └───────────────────────────────────────────────┘  │
│                     │                               │
│                     ▼                               │
│  ┌───────────────────────────────────────────────┐  │
│  │  Secure MCU                                   │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Tamper Response                        │  │  │
│  │  │  1. Zeroize keys                        │  │  │
│  │  │  2. Log event (battery-backed)          │  │  │
│  │  │  3. Disable operation                   │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**実装例（STM32 RTC Tamper）**:

```rust
use stm32l5xx_hal::{pac, rcc::RccExt, rtc::Rtc};

pub struct Stm32TamperDetector {
    rtc: Rtc,
}

impl Stm32TamperDetector {
    pub fn init(rtc: Rtc) -> Self {
        // タンパー検出を有効化
        let dp = unsafe { pac::Peripherals::steal() };

        // TAMP1 (電圧異常), TAMP2 (ケース開放), TAMP3 (温度異常)
        dp.TAMP.cr1.modify(|_, w| {
            w.tamp1e().set_bit()
             .tamp2e().set_bit()
             .tamp3e().set_bit()
        });

        // タンパー検出時にバックアップレジスタを消去
        dp.TAMP.cr2.modify(|_, w| {
            w.tamp1noerase().clear_bit()
             .tamp2noerase().clear_bit()
             .tamp3noerase().clear_bit()
        });

        // 割り込み有効化
        dp.TAMP.ier.modify(|_, w| {
            w.tamp1ie().set_bit()
             .tamp2ie().set_bit()
             .tamp3ie().set_bit()
        });

        Self { rtc }
    }

    pub fn check_tamper(&self) -> Option<TamperEvent> {
        let dp = unsafe { pac::Peripherals::steal() };
        let sr = dp.TAMP.sr.read();

        if sr.tamp1f().bit_is_set() {
            Some(TamperEvent::VoltageAnomaly)
        } else if sr.tamp2f().bit_is_set() {
            Some(TamperEvent::CaseOpen)
        } else if sr.tamp3f().bit_is_set() {
            Some(TamperEvent::TemperatureAnomaly)
        } else {
            None
        }
    }
}
```

---

### Pattern B: メッシュ保護（高セキュリティ）

**用途**: 金融、決済端末
**レベル**: Level 3-4
**コスト**: $50-$200

```
┌─────────────────────────────────────────────────────┐
│  PCB with Security Mesh                             │
│  ┌───────────────────────────────────────────────┐  │
│  │  Top Layer: Ground Plane                      │  │
│  ├───────────────────────────────────────────────┤  │
│  │  Layer 2: Mesh Pattern A                      │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  ╔═══════════════════════════════════╗  │  │  │
│  │  │  ║  ╔═══╗  ╔═══╗  ╔═══╗  ╔═══╗  ╔═╗  ║  │  │  │
│  │  │  ║  ║   ║  ║   ║  ║   ║  ║   ║  ║ ║  ║  │  │  │
│  │  │  ║  ╚═══╝  ╚═══╝  ╚═══╝  ╚═══╝  ╚═╝  ║  │  │  │
│  │  │  ╚═══════════════════════════════════╝  │  │  │
│  │  │  Continuous trace monitored for breaks   │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  ├───────────────────────────────────────────────┤  │
│  │  Layer 3: Secure Components                   │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐       │  │
│  │  │ Crypto  │  │   OTP   │  │  Keys   │       │  │
│  │  └─────────┘  └─────────┘  └─────────┘       │  │
│  ├───────────────────────────────────────────────┤  │
│  │  Layer 4: Mesh Pattern B (orthogonal)         │  │
│  ├───────────────────────────────────────────────┤  │
│  │  Bottom Layer: Ground Plane                   │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  Mesh Monitor IC                              │  │
│  │  • Continuity check (AC injection)            │  │
│  │  • Resistance measurement                     │  │
│  │  • Pattern verification                       │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**部品例**:

| 部品 | 型番 | ベンダー | 機能 |
|------|------|---------|------|
| Mesh Monitor | MAX36010 | Analog Devices | セキュリティメッシュ監視 |
| Secure MCU | MAX32520 | Analog Devices | 統合タンパー検出 |
| 代替 | DS28E83 | Analog Devices | 1-Wire認証+タンパー |

---

### Pattern C: 環境封止（最高セキュリティ）

**用途**: 国防、金融中核
**レベル**: Level 4
**コスト**: $500-$5,000

```
┌─────────────────────────────────────────────────────┐
│  Tamper-Resistant Enclosure                         │
│  ┌───────────────────────────────────────────────┐  │
│  │  Outer Shell (Steel)                          │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  EMI/RFI Shielding                      │  │  │
│  │  │  ┌───────────────────────────────────┐  │  │  │
│  │  │  │  Mesh Layer                       │  │  │  │
│  │  │  │  ┌─────────────────────────────┐  │  │  │  │
│  │  │  │  │  Potting (Epoxy)            │  │  │  │  │
│  │  │  │  │  ┌───────────────────────┐  │  │  │  │  │
│  │  │  │  │  │  PCB + Components    │  │  │  │  │  │
│  │  │  │  │  │  • Crypto module     │  │  │  │  │  │
│  │  │  │  │  │  • Key storage       │  │  │  │  │  │
│  │  │  │  │  │  • Battery backup    │  │  │  │  │  │
│  │  │  │  │  └───────────────────────┘  │  │  │  │  │
│  │  │  │  │  Potting makes decap        │  │  │  │  │
│  │  │  │  │  destructive                │  │  │  │  │
│  │  │  │  └─────────────────────────────┘  │  │  │  │
│  │  │  └───────────────────────────────────┘  │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  Sensors:                                           │
│  • Light detection (photodiode array)               │
│  • X-ray detection                                  │
│  • Drill detection (capacitance change)             │
│  • Acid detection (pH sensor)                       │
└─────────────────────────────────────────────────────┘
```

---

## 3. タンパー応答ロジック

```rust
pub enum TamperSeverity {
    Warning,    // ログのみ
    Critical,   // 鍵消去 + シャットダウン
    Immediate,  // 即座に電源遮断
}

pub struct TamperResponse {
    severity_map: HashMap<TamperEvent, TamperSeverity>,
}

impl TamperResponse {
    pub fn handle(&self, event: TamperEvent) {
        match self.severity_map.get(&event) {
            Some(TamperSeverity::Warning) => {
                // 警告ログのみ
                log::warn!("Tamper warning: {:?}", event);
                self.log_to_nvram(event);
            }
            Some(TamperSeverity::Critical) => {
                // 鍵消去 + シャットダウン
                log::error!("Critical tamper: {:?}", event);
                self.log_to_nvram(event);
                self.zeroize_all_keys();
                self.shutdown();
            }
            Some(TamperSeverity::Immediate) => {
                // 即座に電源遮断（ログなし）
                // ハードウェア回路で実装
                self.hardware_power_cut();
            }
            None => {}
        }
    }

    fn zeroize_all_keys(&self) {
        // すべての鍵スロットをゼロ化
        for slot in 0..MAX_KEY_SLOTS {
            unsafe {
                // 揮発性書き込み（最適化で消えないように）
                core::ptr::write_volatile(
                    KEY_STORAGE_BASE.add(slot * KEY_SIZE) as *mut u8,
                    0,
                );
            }
        }

        // バックアップレジスタもゼロ化
        self.clear_backup_registers();
    }
}
```

---

## 4. バッテリーバックアップ

タンパー検出は電源喪失時も動作する必要がある。

```
┌─────────────────────────────────────────────────────┐
│  Battery Backup System                              │
│  ┌───────────────────────────────────────────────┐  │
│  │  Primary Power ─┬──────────────────────────►  │  │
│  │                 │                             │  │
│  │           ┌─────┴─────┐                       │  │
│  │           │  Charger  │                       │  │
│  │           └─────┬─────┘                       │  │
│  │                 │                             │  │
│  │           ┌─────┴─────┐                       │  │
│  │           │  Battery  │ (Li-MnO2, 10年寿命)   │  │
│  │           │  3.0V     │                       │  │
│  │           └─────┬─────┘                       │  │
│  │                 │                             │  │
│  │           ┌─────┴─────┐                       │  │
│  │           │Power MUX  │                       │  │
│  │           └─────┬─────┘                       │  │
│  │                 │                             │  │
│  │                 ▼                             │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Always-on Domain                       │  │  │
│  │  │  • Tamper detection                     │  │  │
│  │  │  • RTC                                  │  │  │
│  │  │  • Key storage (SRAM)                   │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 5. ベンダー多様性

| 機能 | プライマリ | セカンダリ |
|------|-----------|-----------|
| Secure MCU | MAX32520 (ADI) | STM32L5 (ST) |
| Mesh Monitor | MAX36010 (ADI) | カスタム設計 |
| バッテリー | Tadiran TL-5920 | Panasonic BR-2/3A |

---

## 参照文書

- `chinju/hardware/REQUIREMENTS.md` - 要件
- `chinju/hardware/SECURITY_LEVELS.md` - レベル定義
- FIPS 140-2/3 - Physical Security Requirements
- PCI PTS POI - Device Security Requirements
