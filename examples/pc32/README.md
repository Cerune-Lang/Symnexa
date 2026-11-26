# 再配置を書き込んだ4バイトを照合する

[English](README.en.md)

[apply.ceru](apply.ceru) は `input.o` の再配置表とシンボル表から、書込み位置 `P`、参照先シンボルの位置 `S`、加数 `A` を読みます。`R_X86_64_PC32` の値は `S + A - P` です。書込み位置は `.text` 内にあり、参照先の `anchor` は `.text` または読み取り専用の `PROGBITS` セクションにあります。

| [入力](input.S) | 式 | 書き込む4バイト | ELFの入口 |
| --- | --- | --- | --- |
| 前方にある `anchor` | `8 - 4 - 2 = 2` | `02 00 00 00` | `0xffffffff80001001` |
| 後方にある `anchor` | `0 - 4 - 3 = -7` | `f9 ff ff ff` | `0xffffffff80001002` |
| `.rodata` 内、コード長8または4,096 | `4096 + 19 - 4 - 2 = 4109` | `0d 10 00 00` | `0xffffffff80001001` |
| `.rodata` 内、コード長4,097 | `8192 + 19 - 4 - 2 = 8205` | `0d 20 00 00` | `0xffffffff80001001` |

式の位置は `.text` の先頭から数えます。Rust版はコードの直後を4,096バイト単位で切り上げ、そこに `.rodata` を配置します。先頭の入力セクション `.rodata.prefix` は3バイトです。続く `.rodata.values` は16バイト揃えなのでオフセット16から始まり、その中のオフセット3にある `anchor` は `.rodata` の先頭から19バイトの位置に来ます。

Ceruneはコードセクションをコピーし、該当する4バイトを書き換えて `text.bin` に新規保存します。表示する入口は、LiseOS用のコード配置先 `0xffffffff80001000` に `main` のセクション内オフセットを足した値です。入力は最大64 KiBのx86-64 ELF64オブジェクトで、`main` を定義する実行可能セクションが1個あり、参照先も同じオブジェクト内に定義されている必要があります。

## 実行する

LinuxでRust、Python 3、Clang、GNU objcopy・nm、バイナリファイルを読めるCeruneを用意し、Symnexaのルートで実行します。CIは [Ceruneのコミット `25a6f8f`](https://github.com/Cerune-Lang/Cerune/commit/25a6f8f2c6b76c411bedeeb61c4b1a759f37fa11) を使います。Rust版の比較を始めた時点のコミットは [`70489b5`](https://github.com/Cerune-Lang/Symnexa/commit/70489b57c072d47776bc6d8d6f578c54520b37bb) です。

```sh
cargo build
python3 examples/pc32/compare.py --cerune /path/to/cerune --symnexa target/debug/symnexa
```

[比較試験](compare.py)は各ケースで1個のオブジェクトを組み立て、Rust版とCeruneのHIR・MIR・SSA・VMに同じバイトを渡します。Rust版の `ET_EXEC` から入口と `.text` を取り出し、Ceruneの表示と保存内容に照合します。`nm -P` でRust版の `anchor` のアドレスを読み、書込み位置・変位・加数から得られる値とも照合します。桁あふれの3例では、32ビットの上限・下限と、セクション間の距離を加えると初めて上限を超える入力を試します。Rust版はエラー終了し、Ceruneの例は `PC32 overflow` を表示して保存前に戻ります。
