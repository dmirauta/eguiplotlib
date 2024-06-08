import math

import pyegui

app = pyegui.EguiCanvas()

app.add_figure("plot set 1", 1, 2)
app.add_figure("plot set 2", 2, 2)

x = [6.28 * i / 100 for i in range(100)]

for j in range(2):
    for k in range(2):
        y = [math.sin(xv + 2 * k + j) for xv in x]
        app.add_line("plot set 1", 0, j, x, y)


for i in range(2):
    for j in range(2):
        for k in range(2):
            i_ = i + 1
            j_ = j + 1
            l = 4 * i_ * j_ + 2 * j_ + k + 1
            y = [math.sin(xv + l) for xv in x]
            y2 = [math.sin(0.5 * xv + 2 * l) for xv in x]
            app.add_line("plot set 2", i, j, y, y2)
