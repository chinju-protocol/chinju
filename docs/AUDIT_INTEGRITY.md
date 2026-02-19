# Audit Integrity Guide

CHINJU sidecar の監査ログ（C6）の改ざん耐性に関する運用メモです。

## 仕組み

1. すべての `AuditLogEntry` は `prev_hash` と `hash` でチェーン化されます。
2. 起動時 (`create_audit_system_with_restore`) に既存ログ全体の整合性を検証します。
3. 追記時は直前エントリとの連続性（`sequence` / `prev_hash`）を検証します。
4. ローテーション時はアーカイブの SHA-256 を計算し、任意で HMAC 署名を生成します。

## 設定

`CHINJU_AUDIT_ARCHIVE_HMAC_KEY` を設定すると、ローテーション時に
`archive/*.sig` ファイルが生成されます。

```bash
export CHINJU_AUDIT_ARCHIVE_HMAC_KEY="long-random-secret"
```

`.sig` ファイルには以下が記録されます。

- `archive_file`
- `archive_hash`
- `algorithm` (`hmac-sha256`)
- `signature`
- `generated_at`

## 失敗時の挙動

- 起動時整合性検証に失敗した場合: Sidecar 起動は失敗します（fail-closed）。
- 追記時連続性検証に失敗した場合: 書き込みは拒否されます。

## 関連コード

- `chinju-sidecar/src/services/audit/chain.rs`
- `chinju-sidecar/src/services/audit/persister.rs`
- `chinju-sidecar/src/services/audit/storage/mod.rs`
- `chinju-sidecar/src/services/audit/storage/file.rs`
