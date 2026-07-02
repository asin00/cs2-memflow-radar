CS2 Memflow Radar (Rust)

基于 [rabume/cs2-dma-radar](https://github.com/rabume/cs2-dma-radar) 使用 Rust + memflow 重构的 CS2 雷达

编译
cargo build --release

运行，使用qemu连接器

SETPTRACE=1 cargo run --release -- -c qemu -o win32

若出现无法找到DTB的报错，建议使用kvm连接器

SETPTRACE=1 cargo run --release -- -c kvm -o win32

致谢

本项目基于 rabume/cs2-dma-radar 重构，感谢原作者的工作。
免责声明

本项目仅供学习研究，严禁用于 CS2 官方匹配。使用后果由用户自行承担。

开源协议
MIT License
