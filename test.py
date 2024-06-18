import math

from eguiplotlib import FigHandle, FigureCanvas

canvas = FigureCanvas()

#     vvv type annotation may be required here --------------
fig1: FigHandle = canvas.add_figure("plot set 1", ncols=2)
fig2: FigHandle = canvas.add_figure("plot set 2", 2, 2)

x = [6.28 * i / 100 for i in range(100)]

for j in range(2):
    for k in range(2):
        #           vvv for hint to be available here -------
        plot = fig1.plot(col=j)
        y = [math.sin(xv + 2 * k + j) for xv in x]
        plot.add_line(x, y)


for i in range(2):
    for j in range(2):
        plot = fig2.plot(i, j)
        (i_, j_) = (i + 1, j + 1)
        for k in range(2):
            l_ = 4 * i_ * j_ + 2 * j_ + k + 1
            y = [math.sin(xv + l_) for xv in x]
            y2 = [math.sin(0.5 * xv + 2 * l_) for xv in x]
            plot.add_line(y, y2)
