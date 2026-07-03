# CS2 Memflow Radar

基于 [rabume/cs2-dma-radar](https://github.com/rabume/cs2-dma-radar) 使用 Rust + memflow 重构的 CS2 雷达

## 运行环境
参考
### 宿主机系统 Debian 13.5
### qemu 11.0.1
### 虚拟机系统 Windows 10 ltsc 2019

## 克隆
git clone https://github.com/asin00/cs2-memflow-radar.git



## 后端编译

```bash
cd cs2-memflow-radar/

cargo build --release
```

## 前端编译
```bash
cd cs2-memflow-radar/client

npm install

npm run build
```

## 运行

先启动虚拟机并运行CS2

在windows中使用ssh连接宿主机

使用 QEMU 连接器：

```bash
SETPTRACE=1 cargo run --release -- -c qemu -o win32
```

若出现无法找到 DTB 的报错，建议使用 KVM 连接器：

```bash
SETPTRACE=1 cargo run --release -- -c kvm -o win32
```
程序运行成功后在同一局域网设备上使用浏览器访问 http://<宿主机IP>:8080

<img width="1706" height="1279" alt="c682f97c21f134caf4ca17674e5ee9a8" src="https://github.com/user-attachments/assets/9b14e824-1c8f-47c4-94ba-0180e5203aac" />

<img width="1280" height="1707" alt="c9f9e97a4efd1193babec1961db9dd08" src="https://github.com/user-attachments/assets/394d50aa-0111-4864-a9d2-7523eedc0a8b" />

## 致谢

本项目基于 [rabume/cs2-dma-radar](https://github.com/rabume/cs2-dma-radar) 重构，感谢原作者的工作！

## 免责声明

本项目仅供学习研究，严禁用于 CS2 官方匹配！使用后果由用户自行承担！

## 开源协议

MIT License
