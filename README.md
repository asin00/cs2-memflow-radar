# CS2 Memflow Radar (Rust)

基于 [rabume/cs2-dma-radar](https://github.com/rabume/cs2-dma-radar) 使用 Rust + memflow 重构的 CS2 雷达。

### 编译

```bash
cargo build --release

### 运行

使用 QEMU 连接器：
bash

SETPTRACE=1 cargo run --release -- -c qemu -o win32

若出现无法找到 DTB 的报错，建议使用 KVM 连接器：
bash

SETPTRACE=1 cargo run --release -- -c kvm -o win32

### 致谢

本项目基于 rabume/cs2-dma-radar 重构，感谢原作者的工作。

### 免责声明
本项目仅供学习研究，严禁用于 CS2 官方匹配。使用后果由用户自行承担。

### 开源协议
MIT License
