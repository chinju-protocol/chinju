# Contributing to CHINJU

CHINJUプロジェクトへの貢献に興味を持っていただきありがとうございます。

## 貢献の種類

### 1. GPU環境での検証 (難易度: ⭐)

`chinju-vllm` パッケージはCPU環境でテスト済みですが、実際のGPU環境での検証が必要です。

```bash
# 環境要件
# - CUDA 11.8+ / 12.x
# - vLLM 0.4+
# - Llama-2-7B または Llama-3-8B モデル

cd chinju-vllm
pip install -e ".[dev]"

# テスト実行
pytest tests/ -v

# 結果をIssueで報告
```

報告いただきたい内容:
- GPU型番、CUDAバージョン
- vLLMバージョン
- テスト結果（pass/fail）
- エラーログ（あれば）

### 2. ドキュメント改善 (難易度: ⭐)

- typo修正
- 説明の明確化
- 英語ドキュメントの拡充
- 使用例の追加

### 3. TGI統合 (難易度: ⭐⭐)

Text Generation Inference (TGI) 用のアダプター作成。

参考: `chinju-vllm/chinju_vllm/activation_hook.py`

### 4. ハードウェアPoC (難易度: ⭐⭐⭐)

特許明細書に記載されたハードウェア統合のプロトタイプ作成。

参考: `sample/hardware/` 配下の仕様書

対象ハードウェア:
- OTP (Microchip ATECC608B等)
- HSM (YubiHSM 2, AWS CloudHSM)
- QRNG (ID Quantique Quantis)
- TPM 2.0 (Infineon SLB9670)

## 開発環境のセットアップ

### Rust (chinju-sidecar, chinju-core)

```bash
# Rust 1.75+
rustup update stable

cd chinju-sidecar
cargo build
cargo test
```

### Python (chinju-vllm)

```bash
# Python 3.10+
cd chinju-vllm
pip install -e ".[dev]"

# テスト
pytest tests/ -v

# 型チェック
mypy chinju_vllm/

# リント
ruff check chinju_vllm/
```

## コーディング規約

### Rust

- `cargo fmt` でフォーマット
- `cargo clippy` で警告をゼロに

### Python

- `ruff` でリント
- `mypy --strict` で型チェック
- docstring必須（公開API）

## プルリクエストの手順

1. Issueで作業内容を宣言（重複防止）
2. フォークしてブランチ作成
3. 変更をコミット
4. テストがパスすることを確認
5. PRを作成

### コミットメッセージ

```
<type>: <summary>

<body>
```

type: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`

例:
```
feat: Add TGI activation hook

Implements activation extraction for Text Generation Inference,
mirroring the existing vLLM implementation.
```

## ライセンス

貢献いただいたコードは Apache License 2.0 の下でライセンスされます。

## 質問・相談

- GitHub Issues: バグ報告、機能リクエスト
- GitHub Discussions: 設計相談、質問

## 行動規範

- 敬意を持ったコミュニケーション
- 建設的なフィードバック
- 多様な視点の尊重

---

皆様の貢献をお待ちしています！
