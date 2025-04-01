# 连接到OpenOCD的GDB服务器
target remote :3333

# 打印ITM消息到stdout
monitor tpiu config internal itm.txt uart off 8000000

# 启用ITM端口0
monitor itm port 0 on

# 加载SVD文件
set tdesc filename .vscode/STM32F407.svd

# 加载固件
load

# 在main函数处设置断点
break main

# 继续执行
continue