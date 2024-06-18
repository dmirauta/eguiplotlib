'''A matplotlib style plotting library through egui'''

class FigHandle:
    def plot (self, /, row=0, col=0):
      '''Aquire a handle to the plot at grid coords (i, j).'''
    ...

class FigureCanvas:
    def add_figure (self, /, name, nrows=1, ncols=1):
      '''Add a figure containing a grid of plots to the canvas.'''
    ...
    def is_running (self, /):
      '''Check if egui canvas window is still open.'''
    ...

class PlotHandle:
    def add_line (self, /, x, y):
      '''Add a line to this plot.'''
    ...
