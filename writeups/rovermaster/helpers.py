```python
CHOOSE=1
SEND=2
EXEC=3

G_PLT=0
S_PLT=1
G_NA=2
S_NA=3
M_RO=4
F_INF=5

def joke(j, sz=None):
  if sz is None:
    sz = len(j)
  assert sz <= 0x20, 'joke to big'
  sla('Joke size:', sz)
  sla('Joke:', j)


def opt(o):
  sla('Option: ', o)

def choose(idx):
  assert idx < 0xf, 'rover idx to big'
  opt(CHOOSE)
  sla('rover: ', idx)

def send(cmd):
  opt(SEND)
  sla('action: ', cmd)

def exc():
  opt(EXEC)

def set_name(name, sz=None, oob=False):
  if sz is None:
    sz = len(name)
    if oob:
      sz -= 1

  assert sz <= 0x100, 'name to big'
  send(S_NA)
  exc()
  sla('size:', sz)
  if oob:
    sa('name:', name)
  else:
    sla('name:', name)

def set_planet(name, sz=None):
  if sz is None:
    sz = len(name)

  assert sz <= 0x100, 'planet to big'
  send(S_PLT)
  exc()
  sla('size:', sz)
  sla('planet:', name)

t = get_target()

# exploit goes here

it()
```
