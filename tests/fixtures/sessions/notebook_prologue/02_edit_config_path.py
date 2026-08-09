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
# # config 파일 이름을 고친 경우 — opaque가 prefix 안에 있을 때
#
# `import *`(index 4)보다 **아래**인 index 5를 고쳤다. opaque한 실행은 여전히
# prefix 안에 있으므로 오염 집합에 들어가지 않는다. 서두 다섯 줄이 그대로
# 재사용되어야 한다.

# %%
import os, sys

# %%
from pathlib import Path

# %%
import pandas as pd

# %%
sys.path.append('..')

# %%
from helpers import *

# %%
CONFIG_FILE = "../config.toml"

# %%
config = parse_config(CONFIG_FILE)

# %%
ROOT_DIR = Path(config['root'])
