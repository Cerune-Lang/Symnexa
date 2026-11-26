# 再配置を書き込んだ4バイトを照合する

[English](README.en.md)

[apply.ceru](apply.ceru) は `input.o` の再配置表とシンボル表から、書込み位置 `P`、参照先シンボルの位置 `S`、加数 `A` を読みます。`R_X86_64_PC32` の値は `S + A - P` です。この例では両方の位置が同じセクション内にあるため、セクション先頭からのオフセットで計算できます。

| [入力](input.S) | 式 | 書き込む4バイト | ELFの入口 |
| --- | --- | --- | --- |
| 前方にある `anchor` | `8 - 4 - 2 = 2` | `02 00 00 00` | `0xffffffff80001001` |
| 後方にある `anchor` | `0 - 4 - 3 = -7` | `f9 ff ff ff` | `0xffffffff80001002` |

Ceruneはコードセクションをコピーし、該当する4バイトを書き換えて `text.bin` に新規保存します。表示する入口は、LiseOS用のコード配置先 `0xffffffff80001000` に `main` のセクション内オフセットを足した値です。入力は最大64 KiBのx86-64 ELF64オブジェクトで、実行可能セクションは1個、その中に `main` と参照先の定義が必要です。

## 実行する

LinuxでRust、Python 3、Clang、GNU objcopy、バイナリファイルを読めるCeruneを用意し、Symnexaのルートで実行します。CIは [Ceruneのコミット `25a6f8f`](https://github.com/Cerune-Lang/Cerune/commit/25a6f8f2c6b76c411bedeeb61c4b1a759f37fa11) を使います。Rust版の比較を始めた時点のコミットは [`70489b5`](https://github.com/Cerune-Lang/Symnexa/commit/70489b57c072d47776bc6d8d6f578c54520b37bb) です。

```sh
cargo build
python3 examples/pc32/compare.py --cerune /path/to/cerune --symnexa target/debug/symnexa
```

[比較試験](compare.py)は各ケースで1個のオブジェクトを組み立て、Rust版とCeruneのHIR・MIR・SSA・VMに同じバイトを渡します。Rust版の `ET_EXEC` から入口と `.text` を取り出し、Ceruneの表示と保存内容に照合します。残りの2例では計算結果が符号付き32ビットの上限を超える場合と下限を下回る場合を試します。Rust版はエラー終了し、Ceruneの例は `PC32 overflow` を表示して保存前に戻ります。
