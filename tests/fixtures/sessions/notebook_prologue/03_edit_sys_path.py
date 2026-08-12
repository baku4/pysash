# ---
# jupyter:
#   jupytext:
#     text_representation:
#       extension: .py
#       format_name: percent
#       format_version: '1.3'
#   kernelspec:
#     display_name: Python 3
#     language: python
#     name: python3
# ---

# %% [markdown]
# # `sys.path` 한 줄을 고친 경우 — opaque가 prefix 밖으로 밀려난다
#
# `import *`(index 4)보다 **위**인 index 3을 고쳤다. 이제 opaque한 실행이 실현 열
# 밖으로 나가므로 오염 집합이 전체가 된다 — 무엇이 망가졌는지 알 수 없어 전부
# 다시 돈다. `SessionDiagnostic::OpaqueResidue`가 붙어야 한다.

# %%
import os, sys

# %%
from pathlib import Path

# %%
import pandas as pd

# %%
sys.path.append('../..')

# %%
from helpers import *

# %%
CONFIG_FILE = "../config.json"

# %%
config = parse_config(CONFIG_FILE)

# %%
ROOT_DIR = Path(config['root'])
