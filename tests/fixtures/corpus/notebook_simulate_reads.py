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
# # 시뮬레이션 read 생성
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Simulate sequences from reference

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
test_config = config['tests']['mapping_simulated']
test_config

# %%
test_root_dir = Path(config['root']) / test_config['path']
test_root_dir.mkdir(parents=True, exist_ok=True) # mkdir
test_root_dir

# %% [markdown]
# ### Input

# %%
ref_config = config['reference']
ref_root_dir = Path(config['root']) / ref_config['path']
ref_fpath_file = ref_root_dir / ref_config['fpath']
ref_fpath_df = pd.read_csv(ref_fpath_file)
ref_fpath_df

# %%
# 짬깐 살펴봄
ref_fpath_df.sort_values('sum_len')['common_name'].values

# %%
ref_fpath_df

# %% [markdown]
# ### Output

# %%
fpath_file = test_root_dir / test_config['fpath']
fpath_file

# %% [markdown]
# ## 1. Define conditions

# %%
qry_root_dir = test_root_dir / 'query'
qry_root_dir.mkdir(exist_ok=True)
qry_root_dir

# %%
sorted_ref_fpath_df = ref_fpath_df.sort_values('sum_len')

def get_target_query_count(sum_len):
    if sum_len < 10_000_000:
        return 10_000_000
    elif sum_len < 500_000_000:
        return 5_000_000
    else:
        return 1_000_000


sorted_ref_fpath_df['target_query_count'] = sorted_ref_fpath_df['sum_len'].apply(get_target_query_count)
sorted_ref_fpath_df

# %%
read_length_list = [300] # [300, 1000]

seed = 0

# %%
data = []

for _, row in sorted_ref_fpath_df.iterrows():
    cname = row['common_name']
    num_seqs = row['target_query_count']
    ref_file = row['fasta']
    
    org_num_seqs = round(num_seqs * 1.5)
    
    for read_length in read_length_list:
        out_fq = qry_root_dir / f"org_{cname}_{read_length}.fq"
        out_sam = qry_root_dir / f"org_{cname}_{read_length}.sam"
        
        cmd = f"mason_simulator --seed {seed} -n {org_num_seqs} \
            --num-threads 8 \
            --illumina-read-length {read_length} \
            --fragment-mean-size {read_length*2} \
            -ir {ref_file} -o {out_fq} -oa {out_sam}"
    
        data.append([cname, read_length, num_seqs, ref_file, out_fq, out_sam, org_num_seqs, cmd])

sim_fpath_df = pd.DataFrame(
    data,
    columns = ['cname', 'read_length', 'num_seqs', 'reference', 'org_query', 'org_sam', 'org_num_seqs', 'sim_cmd'],
)

# %%
sim_fpath_df

# %% [markdown]
# ## 2. Simulate sequences

# %%
cmds_to_run = [
    row['sim_cmd']
    for _, row
    in sim_fpath_df.iterrows()
    if not (row['org_query'].exists() and row['org_sam'].exists())
]
len(cmds_to_run)

# %%
sim_out, sim_elp = execute_command(
    cmds_to_run,
    5,
)

# %%
# sim_elp
13940.056765794754

# %% [markdown]
# ## 3. Extract Non-N sequences

# %%
sim_fpath_df['unamb_query'] = sim_fpath_df['org_query'].apply(
    lambda x: x.with_stem(f"unamb_{x.stem}")
)
sim_fpath_df

# %%
from Bio import SeqIO

def extract_non_n_query(row):
    if not row['unamb_query'].exists():
        with open(row['org_query'], 'r') as in_hdl:
            with open(row['unamb_query'], 'w') as out_hdl:
                for record in SeqIO.parse(in_hdl, "fastq"):
                    non_nc_count = sum([c not in ["A", "T", "G", "C"] for c in record.seq])
                    if non_nc_count != 0:
                        continue
                    SeqIO.write(record, out_hdl, "fastq")

# %%
out, elp = execute_function_pool(
    extract_non_n_query,
    [r for _, r in sim_fpath_df.iterrows()],
    4,
)

# %% [markdown]
# ## 4. Sampling query

# %%
min_phred = 33
max_phred = 76
min_phred, max_phred

# %%
sim_fpath_df[['query', 'answer_sam']] = sim_fpath_df.apply(
    lambda r: (
        qry_root_dir / f"{r['cname']}_{r['read_length']}.fa",
        qry_root_dir / f"{r['cname']}_{r['read_length']}.sam",
    ),
    axis=1,
).apply(pd.Series)
fa_stat = sim_fpath_df['unamb_query'].apply(get_fasta_stat_series)
sim_fpath_df['unamb_num_seqs'] = fa_stat['num_seqs'].astype(int)
sim_fpath_df

# %%
from Bio import SeqIO
import math
import gzip

def sampling_fasta(row):
    def count_phred(phred_scale_counts, score):
        phred_scale_counts[score] += 1
    
    org_qry = row['unamb_query']
    tgt_qry = row['query']
    phread_score_table = tgt_qry.with_suffix('.ps_tab')
    
    if not (tgt_qry.exists() and phread_score_table.exists()):
        phred_scale_list = [q for q in range(min_phred, max_phred)]
        phred_scale_counts = [0 for _ in phred_scale_list]
        
        tgt_num_seqs = row['num_seqs']
        org_num_seqs = row['unamb_num_seqs']

        sampling_ratio = org_num_seqs / tgt_num_seqs

        # current rec idx - next qry idx : 이게 0보다 크면 write
        curr_rec_idx = 0
        next_qry_idx = 0

        with open(tgt_qry, 'w') as out_handle:
            with open(org_qry, 'r') as in_handle:
                iterator = SeqIO.parse(in_handle, 'fastq')
                try:
                    while True:
                        to_skip_qry = math.ceil(next_qry_idx - curr_rec_idx)

                        skipped = 0
                        for _ in range(to_skip_qry):
                            _ = next(iterator)
                            skipped += 1
                        record = next(iterator)
                        [count_phred(phred_scale_counts, q) for q in record.letter_annotations['phred_quality']]
                        
                        SeqIO.write(record, out_handle, 'fasta')
                        curr_rec_idx += (to_skip_qry + 1)
                        next_qry_idx += sampling_ratio
                except StopIteration:
                    curr_rec_idx += skipped
        
        ps_line = ','.join([str(v) for v in phred_scale_counts])
        with open(phread_score_table, 'w') as f:
            f.write(ps_line)

# %%
out, elp = execute_function_pool(
    sampling_fasta,
    [r for _, r in sim_fpath_df.iterrows()],
    5,
)

# %%
# elp
1353.7817480564117

# %%
import math
phred_err_p_list = [math.pow(10, s/(-10)) for s in range(max_phred-min_phred)]

def get_presumed_err_count(ps_tab_file):
    with open(ps_tab_file, 'r') as f:
        l = f.readline()
    phred_scale_counts = [int(v) for v in l.strip().split(',')]
    
    presumed_err_count_list = [
        count * err_p
        for count, err_p
        in zip(phred_scale_counts, phred_err_p_list)
    ]
    presumed_err_count = sum(presumed_err_count_list)
    return presumed_err_count
    
presumed_err_counts = sim_fpath_df['query'].apply(lambda x: x.with_suffix('.ps_tab')).apply(get_presumed_err_count)
err_rates = presumed_err_counts / (sim_fpath_df['num_seqs'] * sim_fpath_df['read_length'])
sim_fpath_df['err_rates'] = err_rates
sim_fpath_df

# %% [markdown]
# ### Make small query

# %%
sampling_ratio = 100

sim_fpath_df['small_query'] = sim_fpath_df['query'].apply(lambda x: x.with_stem(f"small_{x.stem}"))
sim_fpath_df['small_answer_sam'] = sim_fpath_df['answer_sam'].apply(lambda x: x.with_stem(f"small_{x.stem}"))
sim_fpath_df['small_num_seqs'] = sim_fpath_df['num_seqs'].apply(lambda x: round(x/sampling_ratio))
sim_fpath_df

# %%
from Bio import SeqIO

for idx, row in sim_fpath_df.iterrows():
    print(f"# idx {idx}")
    query_file = row['query']
    small_query_file = row['small_query']

    to_skip_qry = 0

    if not small_query_file.exists():
        with open(small_query_file, 'w') as out_handle:
            with open(query_file, 'r') as in_handle:
                iterator = SeqIO.parse(in_handle, 'fasta')
                try:
                    while True:
                        for skipped in range(to_skip_qry):
                            _ = next(iterator)
                        record = next(iterator)
                        SeqIO.write(record, out_handle, 'fasta')
                        to_skip_qry = sampling_ratio - 1
                except StopIteration:
                    to_skip_qry = sampling_ratio - skipped - 1

# %% [markdown]
# ### Make unfold query file for `vargas`

# %%
# from Bio import SeqIO
# from Bio.SeqIO.FastaIO import FastaWriter

# def make_unfold_file(qry_file):
#     unfolded_qry_file = qry_file.with_suffix(".unfolded.fasta")
#     if not unfolded_qry_file.exists():
#         with open(unfolded_qry_file, 'w') as out_hdl:
#             writer = FastaWriter(out_hdl, wrap=0)
#             with open(qry_file, 'r') as in_hdl:
#                 for record in SeqIO.parse(in_hdl, 'fasta'):
#                     writer.write_record(record)

# %%
# qry_files = sim_fpath_df[['query', 'small_query']].values.reshape(-1)

# execute_function_pool(
#     make_unfold_file,
#     qry_files,
#     4,
# )

# %% [markdown]
# ## 5. Generate answer SAM

# %%
import pysam
from Bio import SeqIO

def write_reads_to_sam(input_sam_path, output_sam_path, sampled_query_file):
    if not output_sam_path.exists():
        reads_to_write = [
            record.id
            for record
            in SeqIO.parse(sampled_query_file, 'fasta')
        ]
        reads_to_write.append(None)
        
        idx = 0
        next_read = reads_to_write[idx]

        with pysam.AlignmentFile(input_sam_path, "r") as source:
            with pysam.AlignmentFile(output_sam_path, "w", header=source.header) as destination:
                for read in source:
                    if read.query_name == next_read:
                        destination.write(read)
                        idx += 1
                        next_read = reads_to_write[idx]

# %%
data = []

for _, row in sim_fpath_df.iterrows():
    data.append([row['org_sam'], row['answer_sam'], row['query']])
    data.append([row['org_sam'], row['small_answer_sam'], row['small_query']])

gen_ans_out, gen_ans_elp = execute_function_pool_args(
    write_reads_to_sam,
    data,
    5,
)

# %%
gen_ans_elp

# %% [markdown]
# ## 6. Save

# %%
cols = [
    'cname', 'read_length', 'reference', 'err_rates',
    'num_seqs', 'query', 'answer_sam',
    'small_num_seqs', 'small_query', 'small_answer_sam',
]
sim_fpath_df[cols]

# %%
sim_fpath_df[cols].to_csv(fpath_file, index=False)

# %% [markdown]
# ---
# ## Test

# %% [markdown]
# ### Ref info for sup

# %%
ref_fpath_df[['common_name', 'scientific_name', 'num_seqs', 'sum_len']].sort_values('sum_len')

# %% [markdown]
# ### Query info for sup

# %%
df = sim_fpath_df[cols].copy()
df

# %%
for v in df['num_seqs'].values:
    print(v)

# %%
string = """TB
Ecoli
Yeast
ThaleCress
FruitFly
Rice
Zebrafish
Mouse
Human"""
for v in string.split('\n'):
    print(f'Simulated from {v}')

# %%
string = """Mycobacterium tuberculosis
Escherichia coli
Saccharomyces cerevisiae
Arabidopsis thaliana
Drosophila melanogaster
Oryza sativa
Danio rerio
Mus musculus
Homo sapiens"""
snames = [f"{v1[0]}. {v2}" for v1, v2 in [v.split() for v in string.split('\n')]]
for v in snames:
    print(f'Simulated from {v}')

# %%
get_fasta_stat_series(df['query'][0])
