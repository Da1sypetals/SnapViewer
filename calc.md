世界坐标：放缩到窗口和一样大。

鼠标位置（px）- 屏幕中心位置 = 相对中心距离 px
相对中心距离 px * scale = 相对中心距离 世界坐标
相对中心距离 世界坐标 + window transform translate = 世界坐标

inverse index: 每个三角形是哪个alloc产生出来的
然后就可以通过 像素 -> 三角形index -> alloc index 找到对应alloc

对应函数：
screen2world
find_by_pos

---

1. 将mouse screen pos -> mouse world pos
2. 根据距离ratio，插值出新的center在哪里 world Pos
3. 移动center