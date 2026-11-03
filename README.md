# Symnexa

日本語 | [English](README.en.md)

Symnexaは、[Cerune](https://github.com/Cerune-Lang/Cerune)や[Sashimi](https://github.com/Cerune-Lang/Sashimi)が生成したオブジェクトファイルを結合するための、実験的なリンカのプロジェクトです。

最初の用途は、[LiseOS](https://github.com/Cerune-Lang/LiseOS)のx86-64 ELF64カーネルをリンクすることです。**現在は設計資料のみで、リンカ本体は未実装です。** LiseOSは引き続き`ld.lld`でビルドしています。

## 役割

```text
Cerune → Sashimi → ELF64 Object
                         ↓
                    Symnexa（予定）
                         ↓
                    LiseOS ELF64
                         ↓
                       Limine
```

## 資料

- [ドキュメント一覧](docs/README.md)
- [リンカの実装方針](docs/design/linking.ja.md) — 入力、シンボル解決、配置、出力
- [LiseOSとの連携](docs/integration/liseos.ja.md) — 現在の`ld.lld`と検証方法

## 関連リポジトリ

[Cerune](https://github.com/Cerune-Lang/Cerune) · [Sashimi](https://github.com/Cerune-Lang/Sashimi) · [LiseOS](https://github.com/Cerune-Lang/LiseOS)
