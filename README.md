# CS2 Memflow Radar (Rust)

基于 [rabume/cs2-dma-radar](https://github.com/rabume/cs2-dma-radar) 使用 Rust + memflow 重构的 CS2 雷达。

## 运行环境
参考
### 宿主机系统 debian 13.5
### qemu 11.0.1
### 虚拟机系统 Windows 10 ltsc 2019

## 编译

```bash
cargo build --release
```

## 运行

使用 QEMU 连接器：

```bash
SETPTRACE=1 cargo run --release -- -c qemu -o win32
```

若出现无法找到 DTB 的报错，建议使用 KVM 连接器：

```bash
SETPTRACE=1 cargo run --release -- -c kvm -o win32
```
<img width="1280" height="1707" alt="c9f9e97a4efd1193babec1961db9dd08" src="https://github.com/user-attachments/assets/394d50aa-0111-4864-a9d2-7523eedc0a8b" />

## 致谢

本项目基于 [rabume/cs2-dma-radar](https://github.com/rabume/cs2-dma-radar) 重构，感谢原作者的工作。

## 免责声明

本项目仅供学习研究，严禁用于 CS2 官方匹配。使用后果由用户自行承担。

## 开源协议

MIT License
