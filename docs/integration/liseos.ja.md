# LiseOSとの連携

[English](liseos.en.md) · [資料一覧](../README.md)

Symnexaの最初の利用者は[LiseOS](https://github.com/Cerune-Lang/LiseOS)です。**LiseOSは現在`ld.lld`を使用しており、Symnexaは未実装です。**

## 現在のビルド

```text
kernel/main.ceru
       ↓
tools/object-builder（ホスト用Rust）
       ↓
build/kernel.o
       ↓
ld.lld -T linker.ld
       ↓
build/liseos.elf
       ↓
Limine → QEMU
```

`tools/object-builder`はCeruneの`compile_to_native_object`を呼びます。カーネルとして実行されるRustコードではありません。

## リンク時に必要な値

LiseOSの現行設定（[linker.ld](https://github.com/Cerune-Lang/LiseOS/blob/main/linker.ld)）：

| 項目 | 値 |
| --- | --- |
| 出力 | x86-64 ELF64実行ファイル |
| エントリ | `main` |
| 仮想アドレス開始 | `0xffffffff80000000` |
| セグメント | RX、R、RWの`PT_LOAD` |
| ページ | 4 KiB単位で権限を分ける |
| Limine | リクエスト開始・終了マーカーとbase revision 3 |
| リンク方式 | 静的リンク。未定義シンボルはエラー |

`linker.ld`はデータの位置だけでなく、Limineマーカーも直接書き込んでいます。Symnexaへ移行するときはこの機能の代替が必要です。

## 確認方法

LiseOSのリポジトリで既存のテストを基準にします。

```sh
bash scripts/iso.sh
bash scripts/smoke-qemu.sh
```

`scripts/audit_elf.py`でELF形式、シンボル、入口アドレス、Limineマーカーを検査します。QEMU/GDBではCeruneの`main`への到達を確認します。

Symnexa製ELFでも同じ検査が通ることを確認してから、LiseOSのビルドに組み込みます。現在の`ld.lld`経路は比較・復旧用に残せます。

[リンカ側の実装](../design/linking.ja.md)
