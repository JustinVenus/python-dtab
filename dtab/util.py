import sys

PY3 = sys.version_info.major == 3


if PY3:
  def u(str_input):
    return str(str_input)
else:
  def u(str_input):
    return unicode(str_input).decode("utf-8")
