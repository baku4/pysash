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
# # notebook 서두 — `sys.path` 조작과 `import *`
#
# corpus의 notebook 대부분이 이 모양으로 시작한다. `from helpers import *`는
# 바인딩 집합을 정적으로 알 수 없어 opaque다. prefix 안에 있는 동안은 무해하지만
# (양쪽이 똑같이 실행됐다), prefix 밖으로 밀려나면 오염 집합이 전체가 된다.
#
# `parse_config`는 이 소스 어디에서도 바인딩되지 않는다 —
# `StatementDiagnostic::UnresolvedReference`가 붙어야 하는 자리다.

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
CONFIG_FILE = "../config.json"

# %%
config = parse_config(CONFIG_FILE)

# %%
ROOT_DIR = Path(config['root'])
