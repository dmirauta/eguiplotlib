import math

import pyegui

canvas = pyegui.FigureCanvas()

fig1 = canvas.add_figure("plot set 1", 1, 2)
fig2 = canvas.add_figure("plot set 2", 2, 2)

x = [6.28 * i / 100 for i in range(100)]

for j in range(2):
    for k in range(2):
        plot = fig1.plot(0, j)
        y = [math.sin(xv + 2 * k + j) for xv in x]
        plot.add_line(x, y)


for i in range(2):
    for j in range(2):
        for k in range(2):
            plot = fig2.plot(i, j)
            i_ = i + 1
            j_ = j + 1
            l_ = 4 * i_ * j_ + 2 * j_ + k + 1
            y = [math.sin(xv + l_) for xv in x]
            y2 = [math.sin(0.5 * xv + 2 * l_) for xv in x]
            plot.add_line(y, y2)
