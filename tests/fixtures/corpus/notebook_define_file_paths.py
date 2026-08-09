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
# # read mapping 벤치마크의 파일 경로 정의
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Define File Path

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
# ### Output

# %%
test_root_dir = Path(config['root']) / test_config['path']
test_root_dir.mkdir(exist_ok=True)
fpath_file = test_root_dir / test_config['fpath']
fpath_file

# %% [markdown]
# ## 1. Load reference genomes

# %%
ref_config = config['reference']
ref_root_dir = Path(config['root']) / ref_config['path']
ref_fpath_file = ref_root_dir / ref_config['fpath']
ref_fpath_file

# %%
ref_fpath_df = pd.read_csv(ref_fpath_file)
ref_fpath_df

# %% [markdown]
# ## 2. Load query file path

# %%
seqs_list = [
    'tb_novaseq',
    'tb_miseq',
    'human_minion_r10',
    'human_pachifi',
]

# %%
data = []
for seq in seqs_list:
    v = config['sequence'][seq]
    seq_dir = Path(config['root']) / v['path']
    query_file = seq_dir / v['query']
    small_query_file = seq_dir / v['small_query']
    data.append([seq, query_file, small_query_file])
qry_fpath_df = pd.DataFrame(data, columns=['task', 'query', 'small_query'])

# %%
qry_fpath_df

# %%
def get_qry_stat_df(file_list):
    def get_qry_stat(file):
        stat = get_fasta_stat_series(file)
        res = [
            stat[col]
            for col
            in ['num_seqs', 'min_len', 'avg_len', 'max_len']
        ]
        return res
    data = [get_qry_stat(f) for f in file_list]
    return pd.DataFrame(data, columns = ['num_seqs', 'min_len', 'avg_len', 'max_len'])

# %%
qry_stat_df = get_qry_stat_df(qry_fpath_df['query'].values).add_prefix('query_')
sm_qry_stat_df = get_qry_stat_df(qry_fpath_df['small_query'].values).add_prefix('small_query_')

# %%
qry_fpath_wstat_df = pd.concat([qry_fpath_df, qry_stat_df, sm_qry_stat_df], axis=1)
qry_fpath_wstat_df

# %% [markdown]
# ### Make unfold query file for `vargas`

# %%
from Bio import SeqIO
from Bio.SeqIO.FastaIO import FastaWriter

def make_unfold_file(qry_file):
    unfolded_qry_file = qry_file.with_suffix(".unfolded.fasta")
    if not unfolded_qry_file.exists():
        with open(unfolded_qry_file, 'w') as out_hdl:
            writer = FastaWriter(out_hdl, wrap=0)
            with open(qry_file, 'r') as in_hdl:
                for record in SeqIO.parse(in_hdl, 'fasta'):
                    writer.write_record(record)

# %%
qry_files = qry_fpath_wstat_df[['query', 'small_query']].values.reshape(-1)

execute_function_pool(
    make_unfold_file,
    qry_files,
    4,
)

# %% [markdown]
# ## 3. Assign reference

# %%
def get_ref_tag(task):
    if 'gut' in task:
        return
    elif 'tb' in task:
        return 'H37Rv'
    elif 'human' in task:
        return 'T2T'
    else:
        return

# %%
qry_fpath_wstat_df['ref_tag'] = qry_fpath_wstat_df['task'].apply(get_ref_tag)
qry_fpath_wstat_df

# %%
ref_fasta_file_list = [
    ref_fpath_df[ref_fpath_df['tag'] == t].iloc[0]['fasta']
    for t
    in qry_fpath_wstat_df['ref_tag'].values
]

# %%
fpath_df = qry_fpath_wstat_df.copy()
fpath_df['reference'] = ref_fasta_file_list
fpath_df

# %%
def get_ref_stat(file):
    stat = get_fasta_stat_series(file)
    res = [
        stat[col]
        for col
        in ['num_seqs', 'sum_len']
    ]
    return res

# %%
fpath_df[['reference_num_seqs', 'reference_sum_len']] = fpath_df['reference'].apply(get_ref_stat).apply(pd.Series)

# %%
fpath_df

# %% [markdown]
# ## 4. Save

# %%
fpath_df.to_csv(fpath_file, index=False)

# %% [markdown]
# ---

# %% [markdown]
# ## Metadata for supplementary

# %% [markdown]
# ### Query stat

# %%
sup_stat_df = pd.read_csv(fpath_file)
sup_stat_df

# %%
# prj_acc
sup_stat_df['prj_acc'] = sup_stat_df['task'].apply(
    lambda x: config['sequence'][x]['project_accession']
)
# sequencer_acc
sup_stat_df['sequencer_acc'] = [
    99.86161865448815,
    98.79296874022832,
    95.59121544221145,
    99.80810601452568,
]

# sequencer
sup_stat_df['sequencer'] = [
    'Illumina NovaSeq',
    'Illumina MiSeq',
    'Nanopore MinION',
    'PacBio Sequel II',
]

sup_stat_df = sup_stat_df[[
    'prj_acc', 'ref_tag', 'sequencer', 'query_num_seqs', 'query_min_len', 'query_avg_len', 'query_max_len', 'sequencer_acc'
]]
sup_stat_df

# %% [markdown]
# ### Vis read lengths

# %%
task_seqs_list = [
    'tb_novaseq',
    'tb_miseq',
    'human_minion_r10',
    'human_pachifi',
]

# %%
def get_read_len_df(task_seq):
    seq_conf = config['sequence'][task_seq]
    df = pd.read_csv(Path(config['root']) / seq_conf['path'] / seq_conf['query_fpath'])
    read_length_file_list = df['read_length_file'].values
    
    rlen_df = pd.concat([
        pd.read_csv(f) for f in read_length_file_list
    ]).reset_index(drop=True)
    return rlen_df

# %%
read_len_df_list = [
    get_read_len_df(task_seq)
    for task_seq
    in task_seqs_list
]

# %% [markdown]
# ---

# %%
import plotly.graph_objects as go
from plotly.subplots import make_subplots

fig = make_subplots(
    rows=4,
    cols=1,
    subplot_titles=("DataFrame 1", "DataFrame 2", "DataFrame 3", "DataFrame 4")
)

fig.add_trace(go.Histogram(
    x=read_len_df_list[0]['rlen'],
    name="DF1"
), row=1, col=1)
fig.add_trace(go.Histogram(
    x=read_len_df_list[1]['rlen'],
    name="DF2"
), row=2, col=1)
fig.add_trace(go.Histogram(
    x=read_len_df_list[2]['rlen'],
    name="DF3"
), row=3, col=1)
fig.add_trace(go.Histogram(
    x=read_len_df_list[3]['rlen'],
    name="DF4"
), row=4, col=1)

# Update layout for better appearance
fig.update_layout(title_text="Histograms of rlen from Four DataFrames", barmode='overlay')
fig.update_traces(opacity=0.7)  # reduce opacity to see overlapping bars

# %%
fig.show("png", scale=2)
