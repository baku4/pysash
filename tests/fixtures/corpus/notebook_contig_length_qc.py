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
# # hybrid assembly contig 길이 QC
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Length check

# %%
import os, sys
from pathlib import Path
import json
import subprocess

import pandas as pd
import numpy as np

# %%
import time
import os, subprocess
from multiprocessing import Pool

# Decorator for time check
def time_check(func):
    def wrapper(*args, **kwargs):
        start = time.time()
        res = func(*args, **kwargs)
        end = time.time()
        return (res, end - start)
    return wrapper

# Multiprocessing
def subprocs(cmd):
    return subprocess.run(cmd, shell=True, capture_output=True)
@time_check
def execute_command(cmds, pool_num):
    with Pool(pool_num) as p:
        output = p.map(subprocs, cmds)
    return output

# %% [markdown]
# ## Set configs

# %%
root_dir = Path('/root/TB')

# %%
prj_root_dir = root_dir/ 'ref_addition'
prj_root_dir

# %% [markdown]
# ### input

# %%
hybrid_assem_fa_file = prj_root_dir / "hybrid_assem.fa"
hybrid_assem_fa_file

# %% [markdown]
# ### output

# %%
to_drop_contig_file = prj_root_dir / 'low_len_contigs.csv'
to_drop_contig_file

# %% [markdown]
# ## 1. Parse length

# %%
from Bio import SeqIO

# %%
data = []

with open(hybrid_assem_fa_file, 'r') as in_handle:
    for record in SeqIO.parse(in_handle, 'fasta'):
        
        data.append([record.id, len(record.seq)])

# %%
len_df = pd.DataFrame(data, columns = ['id', 'len'])
len_df

# %%
len_df.describe()

# %% [markdown]
# ## 2. Vis

# %%
import plotly.express as px

# %%
fig = px.histogram(len_df, 'len')

# %%
fig.show()

# %%
for i in range(1, 11):
    min_len = i*250
    print(f"If min length = {min_len} : Drop {sum(len_df['len'] < min_len)} contigs")

# %% [markdown]
# ## 3. Select contigs to drop `(len < 1000)`

# %%
to_drop_contig_file

# %%
to_drop_contigs = len_df[len_df['len'] < 1000]['id']
len(to_drop_contigs)

# %%
# Save
to_drop_contigs.to_csv(to_drop_contig_file, index=False)
