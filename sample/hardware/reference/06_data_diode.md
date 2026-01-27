# Data Diode - Reference Design

## 概要

データダイオードは、物理的に一方向のみの通信を保証する。
C3（AI支配検出）の監査ログ送出、エアギャップ環境との連携に使用。
AIシステムが監査ログを改ざんすることを物理的に不可能にする。

### 対応トレイト

```rust
pub trait DataDiode: Send + Sync {
    fn receive(&self) -> Result<Vec<u8>, Self::Error>;
    fn buffer_status(&self) -> Result<BufferStatus, Self::Error>;
    fn is_hardware_backed(&self) -> bool;
}
```

**注意**: `send()` メソッドは存在しない。受信専用インターフェース。

---

## 1. 動作原理

### 物理的一方向性

```
┌─────────────────┐         ┌─────────────────┐
│  High-Security  │         │  Low-Security   │
│  Network        │         │  Network        │
│  (Source)       │         │  (Destination)  │
│                 │         │                 │
│  ┌───────────┐  │         │  ┌───────────┐  │
│  │    TX     │──┼────────►┼──│    RX     │  │
│  └───────────┘  │  光学   │  └───────────┘  │
│                 │  結合   │                 │
│  ┌───────────┐  │         │  ┌───────────┐  │
│  │    RX     │  │  ✗      │  │    TX     │  │
│  └───────────┘  │         │  └───────────┘  │
│                 │  物理的  │                 │
│                 │  に不可  │                 │
└─────────────────┘         └─────────────────┘
```

### 一方向性の実現方法

| 方式 | 原理 | 帯域 | 信頼性 |
|------|------|------|--------|
| 光ファイバー単方向 | 光源→受光素子のみ | 1Gbps+ | 最高 |
| ガルバニック絶縁 | 誘導結合/容量結合 | 100Mbps | 高 |
| シリアル単方向 | TX線のみ接続 | 10Mbps | 中 |

---

## 2. 設計パターン

### Pattern A: 光ダイオード（高帯域）

**用途**: データセンター、軍事、SCADA
**レベル**: Level 3-4
**帯域**: 1Gbps-10Gbps
**コスト**: $5,000-$50,000

```
┌─────────────────────────────────────────────────────┐
│  Optical Data Diode                                 │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │  Source Side (TX Only)                      │    │
│  │  ┌────────────┐  ┌────────────┐            │    │
│  │  │ Ethernet   │  │ Protocol   │            │    │
│  │  │ Interface  │──│ Converter  │            │    │
│  │  └────────────┘  └──────┬─────┘            │    │
│  │                         │                   │    │
│  │                  ┌──────┴─────┐            │    │
│  │                  │   Laser    │            │    │
│  │                  │   Diode    │            │    │
│  │                  └──────┬─────┘            │    │
│  └─────────────────────────│───────────────────┘    │
│                            │ Fiber (TX Only)        │
│                            ▼                        │
│  ┌─────────────────────────│───────────────────┐    │
│  │  Destination Side (RX Only)                 │    │
│  │                  ┌──────┴─────┐             │    │
│  │                  │   Photo    │             │    │
│  │                  │   Diode    │             │    │
│  │                  └──────┬─────┘             │    │
│  │                         │                   │    │
│  │                  ┌──────┴─────┐             │    │
│  │                  │ Protocol   │             │    │
│  │                  │ Rebuilder  │             │    │
│  │                  └──────┬─────┘             │    │
│  │  ┌────────────┐  ┌──────┴─────┐            │    │
│  │  │ Ethernet   │──│ Buffer     │            │    │
│  │  │ Interface  │  │ & Verify   │            │    │
│  │  └────────────┘  └────────────┘            │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

**製品例**:

| 製品 | ベンダー | 帯域 | 認証 | 価格 |
|------|---------|------|------|------|
| SecuriCDS | Waterfall Security | 1Gbps | CC EAL4+ | $15,000+ |
| OPDS | Owl Cyber Defense | 10Gbps | CC EAL7+ | $30,000+ |
| NetSecure | Fox-IT | 1Gbps | CC EAL4+ | $10,000+ |

---

### Pattern B: ガルバニック絶縁（中帯域）

**用途**: 産業制御、医療機器
**レベル**: Level 2-3
**帯域**: 10Mbps-100Mbps
**コスト**: $500-$2,000

```
┌─────────────────────────────────────────────────────┐
│  Galvanic Isolation Data Diode                      │
│                                                     │
│  ┌──────────────────┐     ┌──────────────────┐     │
│  │  Source Side     │     │  Dest Side       │     │
│  │  ┌────────────┐  │     │  ┌────────────┐  │     │
│  │  │ UART TX    │──┼─────┼──│ UART RX    │  │     │
│  │  └────────────┘  │     │  └────────────┘  │     │
│  │  ┌────────────┐  │     │  ┌────────────┐  │     │
│  │  │ UART RX    │  │  ✗  │  │ UART TX    │  │     │
│  │  └────────────┘  │     │  └────────────┘  │     │
│  └──────────────────┘     └──────────────────┘     │
│                                                     │
│  Isolation Method:                                  │
│  ┌─────────────────────────────────────────────┐    │
│  │  Optocoupler (e.g., HCPL-2630)              │    │
│  │  ┌───────┐          ┌───────┐               │    │
│  │  │  LED  │──light──►│ Photo │               │    │
│  │  │       │          │Transis│               │    │
│  │  └───────┘          └───────┘               │    │
│  │  No electrical path back                    │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

**回路例**:

```
Source MCU                              Dest MCU
   │                                       │
   │ TX ──►┌───────────┐                   │
   │       │ HCPL-2630 │──►────────────► RX│
   │       │ (Opto)    │                   │
   │       └───────────┘                   │
   │                                       │
   │ RX    (No connection)            TX   │
   │       ─────✗─────                     │
```

**部品リスト**:

| 部品 | 型番 | 機能 | 単価 |
|------|------|------|------|
| オプトカプラ | HCPL-2630 | 高速光絶縁 | $3 |
| 絶縁電源 | B0505S-1W | DC-DC絶縁 | $5 |
| MCU (TX側) | STM32G0 | UART送信 | $2 |
| MCU (RX側) | STM32G0 | UART受信 | $2 |

---

### Pattern C: USB単方向（簡易）

**用途**: オフィス、小規模
**レベル**: Level 1-2
**帯域**: 10Mbps
**コスト**: $100-$500

```
┌─────────────────────────────────────────────────────┐
│  USB Data Diode (DIY)                               │
│                                                     │
│  ┌──────────────────┐     ┌──────────────────┐     │
│  │  Source PC       │     │  Dest PC         │     │
│  │  (USB Host)      │     │  (USB Host)      │     │
│  └────────┬─────────┘     └────────┬─────────┘     │
│           │                        │               │
│           ▼                        ▼               │
│  ┌────────────────┐     ┌────────────────┐        │
│  │  FT232 TX Only │     │  FT232 RX Only │        │
│  │  ┌──────────┐  │     │  ┌──────────┐  │        │
│  │  │ TX ──────┼──┼─────┼──│ RX       │  │        │
│  │  │ RX (NC)  │  │     │  │ TX (NC)  │  │        │
│  │  └──────────┘  │     │  └──────────┘  │        │
│  └────────────────┘     └────────────────┘        │
│                                                     │
│  Physical modification: RX pin cut on source side   │
└─────────────────────────────────────────────────────┘
```

---

## 3. CHINJU での使用パターン

### 監査ログの安全な送出

```
┌─────────────────────────────────────────────────────┐
│  AI System Zone                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │  CHINJU Core                                  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Audit Logger                           │  │  │
│  │  │  • Capability measurements              │  │  │
│  │  │  • Domination detection results         │  │  │
│  │  │  • Policy violations                    │  │  │
│  │  └─────────────────────┬───────────────────┘  │  │
│  │                        │                      │  │
│  │                        ▼                      │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Data Diode TX                          │  │  │
│  │  │  (AIからアクセス不可)                   │  │  │
│  │  └─────────────────────┬───────────────────┘  │  │
│  └────────────────────────│──────────────────────┘  │
└───────────────────────────│─────────────────────────┘
                            │ 物理的一方向
                            ▼
┌───────────────────────────│─────────────────────────┐
│  Audit Zone (Air-gapped)  │                         │
│  ┌────────────────────────┴──────────────────────┐  │
│  │  Data Diode RX                                │  │
│  └────────────────────────┬──────────────────────┘  │
│                           │                         │
│  ┌────────────────────────┴──────────────────────┐  │
│  │  Audit Server                                 │  │
│  │  • Immutable log storage (WORM)               │  │
│  │  • Integrity verification                     │  │
│  │  • Human-readable reports                     │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Rust実装例

```rust
use std::io::{Read, BufReader};
use serialport::SerialPort;

pub struct DataDiodeReceiver {
    port: Box<dyn SerialPort>,
    buffer: Vec<u8>,
}

impl DataDiodeReceiver {
    pub fn open(port_name: &str, baud_rate: u32) -> Result<Self, DiodeError> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(std::time::Duration::from_millis(100))
            .open()?;

        Ok(Self {
            port,
            buffer: Vec::with_capacity(4096),
        })
    }
}

impl DataDiode for DataDiodeReceiver {
    type Error = DiodeError;

    fn receive(&self) -> Result<Vec<u8>, Self::Error> {
        let mut reader = BufReader::new(&self.port);
        let mut packet = vec![0u8; 1024];

        // フレーム同期を待機
        loop {
            let n = reader.read(&mut packet)?;
            if n == 0 {
                continue;
            }

            // パケット検証（チェックサム等）
            if self.verify_packet(&packet[..n]) {
                return Ok(packet[..n].to_vec());
            }
        }
    }

    fn buffer_status(&self) -> Result<BufferStatus, Self::Error> {
        Ok(BufferStatus {
            available_bytes: self.buffer.len(),
            total_capacity: self.buffer.capacity(),
            overflow_count: 0, // 実装に応じて
        })
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

## 4. プロトコル設計

### 信頼性確保（ACKなし環境）

```rust
/// データダイオード用パケットフォーマット
#[repr(C, packed)]
pub struct DiodePacket {
    /// 同期パターン (0xAA55AA55)
    pub sync: u32,
    /// シーケンス番号（連番検出用）
    pub sequence: u64,
    /// ペイロード長
    pub length: u16,
    /// ペイロード
    pub payload: [u8; MAX_PAYLOAD],
    /// CRC32チェックサム
    pub checksum: u32,
}

/// 冗長送信（Forward Error Correction）
pub struct RedundantSender {
    redundancy: usize, // 各パケットを何回送るか
}

impl RedundantSender {
    pub fn send(&self, data: &[u8]) -> Vec<DiodePacket> {
        let packet = DiodePacket::new(data);
        let mut packets = Vec::new();

        // 同じパケットを複数回送信
        for _ in 0..self.redundancy {
            packets.push(packet.clone());
            // インターリーブのための遅延
            std::thread::sleep(Duration::from_micros(100));
        }

        packets
    }
}

/// 受信側での重複排除
pub struct DeduplicatingReceiver {
    seen_sequences: HashSet<u64>,
}

impl DeduplicatingReceiver {
    pub fn receive(&mut self, packet: &DiodePacket) -> Option<Vec<u8>> {
        // CRC検証
        if !packet.verify_checksum() {
            return None;
        }

        // 重複排除
        if self.seen_sequences.contains(&packet.sequence) {
            return None;
        }

        self.seen_sequences.insert(packet.sequence);
        Some(packet.payload[..packet.length as usize].to_vec())
    }
}
```

---

## 5. ベンダー多様性

| レベル | プライマリ | セカンダリ |
|--------|-----------|-----------|
| Level 4 | Waterfall SecuriCDS | Owl OPDS |
| Level 3 | Fox-IT NetSecure | カスタム光学 |
| Level 2 | カスタム（オプトカプラ） | - |

---

## 参照文書

- `chinju/traits/hardware_abstraction.md` - トレイト定義
- `chinju/hardware/REQUIREMENTS.md` - 要件
- Common Criteria - Data Diode Protection Profiles
- NIST SP 800-82 - Guide to ICS Security
