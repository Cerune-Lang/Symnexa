# Symnexa

日本語 | [English](README.en.md)

Symnexaは、[Cerune](https://github.com/Cerune-Lang/Cerune)や[Sashimi](https://github.com/Cerune-Lang/Sashimi)が生成したオブジェクトファイルを結合するための、実験的なリンカのプロジェクトです。

最初の用途は、[LiseOS](https://github.com/Cerune-Lang/LiseOS)のx86-64 ELF64カーネルをリンクすることです。**Rust製のELF64リンカを実装中です。** LiseOSの通常ビルドは引き続き`ld.lld`を使用します。Symnexaは独立したCIでLiseOSの起動を検証します。

## 役割

```text
Cerune → Sashimi → ELF64 Object
                         ↓
                    Symnexa
                         ↓
                    LiseOS ELF64
                         ↓
                       Limine
```

## ビルド

Rust/Cargoを用意してください。

```sh
cargo test
cargo run -- link --liseos -o liseos.elf kernel.o
```

`kernel.o`にはCerune/Sashimi生成のELF64オブジェクトを指定します。現時点ではx86-64の静的ELF64、LiseOS専用の配置に対応します。

## 資料

- [ドキュメント一覧](docs/README.md)
- [リンカの実装方針](docs/design/linking.ja.md) — 入力、シンボル解決、配置、出力
- [LiseOSとの連携](docs/integration/liseos.ja.md) — 現在の`ld.lld`と検証方法

## 関連リポジトリ

[Cerune](https://github.com/Cerune-Lang/Cerune) · [Sashimi](https://github.com/Cerune-Lang/Sashimi) · [LiseOS](https://github.com/Cerune-Lang/LiseOS)
