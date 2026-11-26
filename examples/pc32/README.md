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

## 別のオブジェクトの定義を参照する

[pair.ceru](pair.ceru) は `first.o`、`second.o` の順に各入力のコードを配置し、間をゼロで埋めます。[参照側](caller.S)の再配置に記録されたシンボル番号は、そのオブジェクト内の番号です。そこから文字列表で名前を読み、[定義側](provider.S)の公開シンボルの名前と末尾まで照合します。名前が一致すると、定義側コードの配置先にシンボルのオフセットを足して参照先を求められます。

| 入力順 | `.text` 先頭からの参照先・書込み位置 | 変位 | 書き込むバイト | `main` の入口 |
| --- | --- | ---: | --- | --- |
| 参照側 → 定義側 | `19 - 4 - 2` | 13 | `0d 00 00 00` | `0xffffffff80001001` |
| 定義側 → 参照側 | `3 - 4 - 10` | -11 | `f5 ff ff ff` | `0xffffffff80001009` |

参照側は7バイトで8バイト揃え、定義側は5バイトで16バイト揃えです。たとえば定義側を先に置くと、次のコードはオフセット8から始まり、その中のオフセット2を書き換えます。次のコマンドは表の1行目を実行します。新しいディレクトリを使ってください。

```sh
mkdir pair-input
clang --target=x86_64-unknown-linux-gnu -c examples/pc32/caller.S -o pair-input/first.o
clang --target=x86_64-unknown-linux-gnu -c examples/pc32/provider.S -o pair-input/second.o
cerune run examples/pc32/pair.ceru --file-root pair-input
```

この例は各入力に1つの実行可能セクションを使い、その中で定義された名前を扱います。再配置は2入力合わせて1件、入力と保存するコードの上限はそれぞれ64 KiBです。`GLOBAL` と `WEAK` の定義を名前解決に使い、Rust版と同じく同名の公開定義が複数あれば保存前に終了します。未解決や重複のときは理由と該当する名前のバイト列を表示します。比較試験では、入力順の反転、シンボル名の変更、弱い定義、名前の末尾だけが異なる定義、同名のローカル定義、公開名の重複を確認します。

## 入口とロード先を持つELFを保存する

同じ `first.o` と `second.o` を使って次を実行すると、[pair_exec.ceru](pair_exec.ceru) が `pair-input/pair.elf` を新規保存します。

```sh
cerune run examples/pc32/pair_exec.ceru --file-root pair-input
readelf -h -l pair-input/pair.elf
```

`Entry point address` は上表の `main`、`LOAD` 行はファイルのどのバイトをどのアドレスへ置くかを示します。先頭の入力順なら、ファイルのオフセット `0x1000` から21バイトが `0xffffffff80001000` に対応します。読み取り・実行用のセグメント1個でコードを表す例なので、他に配置すべきデータがある入力は保存前に診断します。比較試験はRust版ELFから同じ権限のセグメントを選び、入口、ロード先、長さ、整列、再配置後の全バイトを照合します。LiseOSの起動には、今後Limineの要求データを含むセグメントも必要です。

## 実行する

LinuxでRust、Python 3、Clang、GNU objcopy・nm、バイナリファイルを読めるCeruneを用意し、Symnexaのルートで実行します。CIは [Ceruneのコミット `25a6f8f`](https://github.com/Cerune-Lang/Cerune/commit/25a6f8f2c6b76c411bedeeb61c4b1a759f37fa11) を使います。Rust版の比較を始めた時点のコミットは [`70489b5`](https://github.com/Cerune-Lang/Symnexa/commit/70489b57c072d47776bc6d8d6f578c54520b37bb) です。

```sh
cargo build
python3 examples/pc32/compare.py --cerune /path/to/cerune --symnexa target/debug/symnexa
```

[比較試験](compare.py)はケースごとに入力オブジェクトを組み立て、Rust版とCeruneのHIR・MIR・SSA・VMに同じバイトを渡します。Rust版の `ET_EXEC` から入口と `.text` を取り出し、Ceruneの表示と保存内容に照合します。`nm -P` でRust版の `anchor` のアドレスを読み、書込み位置・変位・加数から得られる値とも照合します。桁あふれの3例では、32ビットの上限・下限と、セクション間の距離を加えると初めて上限を超える入力を試します。Rust版はエラー終了し、Ceruneの例は `PC32 overflow` を表示して保存前に戻ります。
