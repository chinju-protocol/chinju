# TEE (Trusted Execution Environment) - Reference Design

## 概要

TEEは、OS・ハイパーバイザからも隔離された実行環境を提供する。
C1（HCAL）の能力測定、C3（支配検出）の計算をAIシステムから隔離するために必須。

### 対応トレイト

```rust
pub trait SecureExecution: Send + Sync {
    fn execute_in_enclave<F, R>(&self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce() -> R + Send,
        R: Send;
    fn remote_attestation(&self) -> Result<AttestationReport, Self::Error>;
    fn seal(&self, data: &[u8]) -> Result<SealedData, Self::Error>;
    fn unseal(&self, sealed: &SealedData) -> Result<Vec<u8>, Self::Error>;
    fn is_hardware_backed(&self) -> bool;
}
```

---

## 1. 設計パターン

### Pattern A: Intel SGX（サーバー向け）

**用途**: クラウド、データセンター
**レベル**: Level 3-4
**プラットフォーム**: Intel Xeon (Ice Lake以降)

```
┌─────────────────────────────────────────────────────┐
│  Host OS (Linux)                                    │
│  ┌───────────────────────────────────────────────┐  │
│  │  Untrusted Application                        │  │
│  │  (CHINJU Host Layer)                          │  │
│  └───────────────────────┬───────────────────────┘  │
│                          │ ECALL                    │
│  ════════════════════════╪══════════════════════════│
│                          ▼                          │
│  ┌───────────────────────────────────────────────┐  │
│  │  SGX Enclave (Trusted)                        │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  CHINJU Core Logic                      │  │  │
│  │  │  • Capability Measurement (C1)          │  │  │
│  │  │  • Domination Detection (C3)            │  │  │
│  │  │  • Threshold Verification (C4)          │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Sealed Storage                         │  │  │
│  │  │  (Enclave-specific key)                 │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
│                          │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  SGX Driver (intel_sgx.ko)                    │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                          │
┌─────────────────────────┴───────────────────────────┐
│  Intel ME / CSME                                    │
│  (Hardware Root of Trust)                           │
└─────────────────────────────────────────────────────┘
```

**Rust実装（rust-sgx-sdk）**:

```rust
// enclave/Cargo.toml
[dependencies]
sgx_tstd = { git = "https://github.com/apache/incubator-teaclave-sgx-sdk" }
sgx_tcrypto = { git = "..." }

// enclave/src/lib.rs
#![no_std]
use sgx_tstd as std;

#[no_mangle]
pub extern "C" fn ecall_measure_capability(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_len: *mut usize,
) -> i32 {
    // エンクレーブ内で能力測定を実行
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };

    // C1能力測定ロジック（AIからアクセス不可）
    let result = chinju_core::measure_capability(input);

    // 結果を返却
    unsafe {
        std::ptr::copy_nonoverlapping(result.as_ptr(), output_ptr, result.len());
        *output_len = result.len();
    }

    0 // Success
}

#[no_mangle]
pub extern "C" fn ecall_get_attestation_report(
    report_ptr: *mut u8,
    report_len: *mut usize,
) -> i32 {
    // リモートアテステーション用のレポートを生成
    let report = sgx_tcrypto::create_report()?;
    // ...
    0
}
```

**ホスト側実装**:

```rust
use sgx_urts::SgxEnclave;

pub struct SgxSecureExecution {
    enclave: SgxEnclave,
}

impl SecureExecution for SgxSecureExecution {
    type Error = SgxError;

    fn execute_in_enclave<F, R>(&self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        // ECALLでエンクレーブ内関数を呼び出し
        // 注: 実際にはクロージャを直接渡せないため、
        // シリアライズされたデータと関数IDを渡す
        self.enclave.call(EnclaveFunctionId::Execute, f.serialize())?
    }

    fn remote_attestation(&self) -> Result<AttestationReport, Self::Error> {
        let mut report = vec![0u8; 1024];
        let mut report_len = 0usize;

        unsafe {
            ecall_get_attestation_report(
                self.enclave.geteid(),
                report.as_mut_ptr(),
                &mut report_len,
            )?;
        }

        Ok(AttestationReport::parse(&report[..report_len])?)
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

### Pattern B: ARM TrustZone（組み込み向け）

**用途**: IoT、エッジデバイス、モバイル
**レベル**: Level 2-3
**プラットフォーム**: ARM Cortex-A/M (TrustZone対応)

```
┌─────────────────────────────────────────────────────┐
│  ARM Cortex-M33 / M55 (TrustZone-M)                 │
│  ┌─────────────────────┬─────────────────────────┐  │
│  │  Non-Secure World   │  Secure World           │  │
│  │  ┌───────────────┐  │  ┌───────────────────┐  │  │
│  │  │ Application   │  │  │ Trusted App       │  │  │
│  │  │ (FreeRTOS)    │  │  │ (CHINJU Core)     │  │  │
│  │  └───────────────┘  │  └───────────────────┘  │  │
│  │  ┌───────────────┐  │  ┌───────────────────┐  │  │
│  │  │ NS Driver     │──┼──│ TF-M / OP-TEE    │  │  │
│  │  └───────────────┘  │  └───────────────────┘  │  │
│  └─────────────────────┴─────────────────────────┘  │
│  ┌───────────────────────────────────────────────┐  │
│  │  Secure Attribution Unit (SAU)                │  │
│  │  Memory Protection Unit (MPU)                 │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Rust実装（TF-M PSA API）**:

```rust
// Secure側（TF-M Secure Partition）
#![no_std]
use psa_crypto::*;

#[no_mangle]
pub extern "C" fn chinju_psa_call(
    in_vec: *const psa_invec,
    in_len: u32,
    out_vec: *mut psa_outvec,
    out_len: u32,
) -> psa_status_t {
    // PSA API経由でSecure Worldの関数を呼び出し
    let input = unsafe { read_invec(in_vec, in_len) };

    let result = match input.function_id {
        FUNC_MEASURE_CAPABILITY => measure_capability(&input.data),
        FUNC_DETECT_DOMINATION => detect_domination(&input.data),
        _ => return PSA_ERROR_NOT_SUPPORTED,
    };

    unsafe { write_outvec(out_vec, out_len, &result) };
    PSA_SUCCESS
}

// Non-Secure側
pub struct TrustZoneSecureExecution;

impl SecureExecution for TrustZoneSecureExecution {
    type Error = PsaError;

    fn execute_in_enclave<F, R>(&self, f: F) -> Result<R, Self::Error> {
        // PSA Client API経由でSecure Partitionを呼び出し
        psa_call(CHINJU_SERVICE_SID, PSA_IPC_CALL, &input, &mut output)?;
        Ok(R::deserialize(&output))
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

### Pattern C: AMD SEV（クラウド向け）

**用途**: AMD EPYC サーバー、機密コンピューティング
**レベル**: Level 3
**プラットフォーム**: AMD EPYC (SEV/SEV-ES/SEV-SNP)

```
┌─────────────────────────────────────────────────────┐
│  Hypervisor (KVM + QEMU)                            │
│  ┌───────────────────────────────────────────────┐  │
│  │  SEV-SNP Encrypted VM                         │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  CHINJU Application                     │  │  │
│  │  │  (Full Linux environment)               │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  │  Memory: AES-128 Encrypted                    │  │
│  │  Register state: Protected (SEV-ES)           │  │
│  │  Integrity: Validated (SEV-SNP)               │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                          │
┌─────────────────────────┴───────────────────────────┐
│  AMD Secure Processor (SP)                          │
│  (Hardware Root of Trust)                           │
└─────────────────────────────────────────────────────┘
```

**Rust実装**:

```rust
use sev::certs::Verifier;
use sev::firmware::Firmware;

pub struct SevSecureExecution {
    firmware: Firmware,
}

impl SecureExecution for SevSecureExecution {
    type Error = SevError;

    fn remote_attestation(&self) -> Result<AttestationReport, Self::Error> {
        // SEV-SNP Attestation Report取得
        let report = self.firmware.get_report()?;

        // AMD Root of Trustで検証
        let chain = self.firmware.get_certificate_chain()?;
        chain.verify(&report)?;

        Ok(AttestationReport {
            platform_id: report.chip_id.to_vec(),
            measurement: report.measurement.to_vec(),
            signature: report.signature.to_vec(),
        })
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

### Pattern D: AWS Nitro Enclaves

**用途**: AWSクラウドネイティブ
**レベル**: Level 3
**プラットフォーム**: AWS EC2 (Nitro対応インスタンス)

```
┌─────────────────────────────────────────────────────┐
│  EC2 Instance (Parent)                              │
│  ┌───────────────────────────────────────────────┐  │
│  │  Application                                  │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  vsock client                           │  │  │
│  │  └─────────────────────┬───────────────────┘  │  │
│  └────────────────────────│──────────────────────┘  │
│                           │ vsock                   │
│  ┌────────────────────────│──────────────────────┐  │
│  │  Nitro Enclave         ▼                      │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │  Enclave Application                    │  │  │
│  │  │  (CHINJU Core)                          │  │  │
│  │  │  • Isolated memory                      │  │  │
│  │  │  • No persistent storage                │  │  │
│  │  │  • No network access                    │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                          │
┌─────────────────────────┴───────────────────────────┐
│  Nitro Hypervisor                                   │
│  (Hardware-enforced isolation)                      │
└─────────────────────────────────────────────────────┘
```

**Rust実装**:

```rust
use aws_nitro_enclaves_cose::CoseSign1;
use nsm_io::{Request, Response};

pub struct NitroSecureExecution {
    nsm_fd: i32,  // Nitro Secure Module FD
}

impl SecureExecution for NitroSecureExecution {
    type Error = NitroError;

    fn remote_attestation(&self) -> Result<AttestationReport, Self::Error> {
        // NSMから attestation document 取得
        let request = Request::Attestation {
            user_data: Some(vec![/* ... */]),
            nonce: Some(generate_nonce()),
            public_key: None,
        };

        let response = nsm_io::send_request(self.nsm_fd, request)?;

        match response {
            Response::Attestation { document } => {
                // COSE_Sign1形式のドキュメントを解析
                let cose = CoseSign1::from_bytes(&document)?;
                Ok(AttestationReport::from_cose(cose))
            }
            _ => Err(NitroError::UnexpectedResponse),
        }
    }

    fn is_hardware_backed(&self) -> bool {
        true
    }
}
```

---

## 2. ベンダー多様性

| 要件 | プライマリ | セカンダリ | ターシャリ |
|------|-----------|-----------|-----------|
| サーバー | Intel SGX | AMD SEV | AWS Nitro |
| エッジ | ARM TrustZone | RISC-V PMP | - |
| クラウド | AWS Nitro | Azure CC | GCP CC |

---

## 3. リモートアテステーション

### 検証フロー

```
1. Enclave → 生成: Attestation Report
2. Report → 送信 → Verifier
3. Verifier → 取得: Platform Certificate Chain
4. Verifier → 検証:
   - Certificate Chain (Root of Trust)
   - Measurement (MRENCLAVE/MRSIGNER)
   - Security Version
   - Debug Flag = false
5. Verifier → 判定: 信頼可能か
```

### CHINJU固有の検証項目

```rust
pub struct ChinJuAttestationPolicy {
    /// 許可されたMRENCLAVE値のリスト
    pub allowed_measurements: Vec<[u8; 32]>,
    /// 最小セキュリティバージョン
    pub min_security_version: u16,
    /// デバッグモード禁止
    pub require_production_mode: bool,
    /// 署名者の公開鍵
    pub expected_signer: PublicKey,
}

impl ChinJuAttestationPolicy {
    pub fn verify(&self, report: &AttestationReport) -> Result<(), PolicyError> {
        // 測定値の検証
        if !self.allowed_measurements.contains(&report.measurement) {
            return Err(PolicyError::UnknownMeasurement);
        }

        // セキュリティバージョンの検証
        if report.security_version < self.min_security_version {
            return Err(PolicyError::OutdatedVersion);
        }

        // 本番モードの検証
        if self.require_production_mode && report.debug_mode {
            return Err(PolicyError::DebugModeNotAllowed);
        }

        Ok(())
    }
}
```

---

## 参照文書

- `chinju/traits/hardware_abstraction.md` - トレイト定義
- `chinju/hardware/REQUIREMENTS.md` - 要件
- Intel SGX Developer Reference
- ARM TrustZone Technology Overview
- AMD SEV-SNP ABI Specification
- AWS Nitro Enclaves User Guide
