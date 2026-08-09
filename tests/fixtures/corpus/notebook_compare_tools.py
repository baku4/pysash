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
# # 정렬 도구 성능 비교 표 만들기
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Comparing the results

# %%
import os, sys
from pathlib import Path
import json

import pandas as pd
import numpy as np

# %%
# import utils.py
currentdir = os.path.dirname(os.path.realpath(__name__))
parentdir = os.path.dirname(currentdir)
sys.path.append(parentdir)

from utils import *

# %% [markdown]
# ## Parse configs

# %%
config_file = "../config.json"
config = parse_config(config_file)

# %%
test_config = config['tests']['read_mapping']
test_config

# %% [markdown]
# ### Input

# %%
test_root_dir = Path(config['root']) / test_config['path']
test_root_dir

# %%
sig_root_dir = test_root_dir / 'sigalign'
sig_results_file = sig_root_dir / 'results.csv'
sig_results_file

# %%
others_root_dir = test_root_dir / 'other_tools'
others_results_file = others_root_dir / 'results.csv'
others_results_file

# %% [markdown]
# ### Output

# %% [markdown]
# ## 1. Load and merge data

# %%
df_list = []

# sigalign
df = pd.read_csv(sig_results_file)
df = df.rename(
    columns = {
        'total_time': 'time',
    }
)
df['is_small_query'] = False
df['tool'] = df['cutoff'].apply(lambda x: f"SigAlign ({x})")
df_list.append(df)

# other tools
others_results_df = pd.read_csv(others_results_file)
for _, row in others_results_df.iterrows():
    df = pd.read_csv(row['merged_result'])
    df['task'] = row['task']
    df['query_num_seqs'] = df['is_small_query'].apply(
        lambda x: row['small_query_num_seqs'] if x else row['query_num_seqs']
    )
    df['reference_sum_len'] = row['reference_sum_len']
    df_list.append(df)

# %%
merged_df = pd.concat(df_list, axis=0).reset_index(drop=True)
merged_df = merged_df[[
    'tool', 'task', 'reference_sum_len', 'refgen_time',
    'is_small_query', 'query_num_seqs', 'time',  'qry_count', 
    'ident', 'length'
]]
merged_df

# %% [markdown]
# ## 2. Encode stat

# %%
df = merged_df.copy()

df['throughput'] = df['query_num_seqs'] / df['time']
df['pident'] = 100 * df['ident'] / df['length']
df['coverage'] = df['length'] / df['reference_sum_len']
df['mapping_rate'] = 100 * df['qry_count'] / df['query_num_seqs']
df['output_per_mapped_read'] = df['length'] / df['qry_count']

encoded_df = df[[
    'tool', 'task', 'throughput', 'pident', 'mapping_rate',
]].copy()

# %%
encoded_df

# %% [markdown]
# ## 3. View Table by Task

# %%
task_list = [
    'tb_novaseq',
    'tb_miseq',
    'human_minion_r10',
    'human_pachifi',
]
tool_order = [
    'SigAlign (shallow)',
    'SigAlign (deep)',
    'blastn',
    'mmseqs2',
    'bwa',
    'bowtie2',
    'hisat2',
    'minimap2',
    'razers3_mh',
]
df_list = []

for task in task_list:
    df = encoded_df[encoded_df['task'] == task].drop(columns=['task']).copy()
    df = df.set_index('tool').transpose()[tool_order]
    df = df.reset_index()
    df['task'] = task
    df = df.rename(columns={'index': 'value'})
    df = df.set_index(['task', 'value'])
    df_list.append(df)

trans_df = pd.concat(df_list)
trans_df

# %% [markdown]
# ### Scaling

# %%
df = trans_df.copy()
indexer = lambda x: [(task, x) for task in task_list]

def scale(x):
    if x > 100:
        return "{:,.0f}".format(x)
    elif x <= 1:
        return "{:,.4f}".format(x)
    else:
        return "{:,.2f}".format(x)
    
for col in ['throughput', 'mapping_rate', 'pident']:
    idx = indexer(col)
    df.loc[idx] = df.loc[idx].applymap(scale)
scaled_df = df.copy()
scaled_df

# %% [markdown]
# ---
# # For sup

# %%
sig_ref_df = pd.read_csv(sig_results_file)
sig_ref_df

# %%
df = sig_ref_df[['task', 'cutoff', 'min_len', 'max_ppl']].copy()
df['allowed mismatch per 100bp'] = df['max_ppl'].apply(lambda x: (x*100)/4)
df

# %%
indices = [idx for idx in scaled_df.index if idx[1] == 'pident']
scaled_df.loc[indices]

# %% [markdown]
# ### RazerS 3

# %%
scaled_df[['razers3_mh']]

# %% [markdown]
# ---
# # For sup

# %% [markdown]
# ### predict penalty and ident

# %%
df = sig_results_df.copy()
df['ppl'] = df['penalty'] / df['length']
df

# %%
df[['task', 'min_len', 'max_ppl', 'ppl', 'pident']]
