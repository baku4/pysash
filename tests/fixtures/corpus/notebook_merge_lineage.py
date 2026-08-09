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
# # lineage 결과 병합
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Merge lineage info (GReAT + CRyPTIC)
# from `kvarq`

# %%
import os, sys
from pathlib import Path
import json
import subprocess

import pandas as pd
import numpy as np

# %%
# import utils.py
currentdir = os.path.dirname(os.path.realpath(__name__))
parentdir = os.path.dirname(currentdir)
sys.path.append(parentdir)

from utils import *

# %%
config_file = "../config.toml"
config = parse_config(config_file)

# %% [markdown]
# ### input

# %%
prj_root_dir = Path(config['root_dir'])
prj_root_dir

# %%
gread_wgs_fpath_file = config['data_path']['great']['wgs_fpath']
great_lineage_file = config['data_path']['great']['wgs_lineage']
gread_wgs_fpath_file, great_lineage_file

# %%
kvarq_root_dir = prj_root_dir / 'kvarq_res'
cryptic_ind_lin_file = kvarq_root_dir / 'ind_lin.csv'
cryptic_pak_lin_file = kvarq_root_dir / 'pak_lin.csv'
cryptic_ind_lin_file, cryptic_pak_lin_file

# %% [markdown]
# ### output

# %%
lin_meta_file = prj_root_dir / config['meta']['lineage']
lin_meta_file

# %% [markdown]
# ## Load lin file

# %%
gread_wgs_fpath_df = pd.read_csv(gread_wgs_fpath_file)
great_lin_df = pd.read_csv(great_lineage_file)
great_lin_df = gread_wgs_fpath_df[['running_id', 'source']].merge(
    great_lin_df, left_on='running_id', right_on='running_id'
)
great_lin_df = great_lin_df.rename(columns={
    "lineage": "kvarq_res",
})
great_lin_df['kvarq_res'] = great_lin_df['kvarq_res'].fillna("")
great_lin_df

# %%
cryptic_ind_lin_df = pd.read_csv(cryptic_ind_lin_file)
cryptic_ind_lin_df['source'] = 'India'
cryptic_pak_lin_df = pd.read_csv(cryptic_pak_lin_file)
cryptic_pak_lin_df['source'] = 'Pakistan'
cryptic_lin_df = pd.concat([cryptic_ind_lin_df, cryptic_pak_lin_df])
cryptic_lin_df = cryptic_lin_df[['running_id', 'source', 'kvarq_res']]
cryptic_lin_df

# %% [markdown]
# ### Merge and encode lineage

# %%
def encode_lineage(kvarq_res):
    if kvarq_res.startswith('lineage 1'):
        return 'L1'
    elif kvarq_res.startswith('lineage 2'):
        return 'L2'
    elif kvarq_res.startswith('lineage 3'):
        return 'L3'
    elif kvarq_res.startswith('lineage 4'):
        return 'L4'
    else:
        print(kvarq_res)
        return 'Unknown'

# %%
df1 = great_lin_df.copy()
df1['prj'] = 'great'
df2 = cryptic_lin_df.copy()
df2['prj'] = 'cryptic'
lin_df = pd.concat([df1, df2])
lin_df['lineage'] = lin_df['kvarq_res'].apply(encode_lineage)

# %%
lin_df = lin_df[['running_id', 'prj', 'source', 'lineage']]
lin_df

# %% [markdown]
# ### Save

# %%
lin_df.to_csv(lin_meta_file, index=False)

# %%
lin_df['lineage'].value_counts()

# %% [markdown]
# ---
# # Test

# %%
great_lin_df['source'].value_counts()

# %%
cryptic_lin_df['source'].value_counts()

# %%
lin_df

# %%
lin_df['source'].value_counts()

# %%
source_df = lin_df[['prj', 'source']].copy()
source_df = source_df.reset_index(drop=True)
source_df
