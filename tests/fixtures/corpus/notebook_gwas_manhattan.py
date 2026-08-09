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
# # GWAS 결과 manhattan plot
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Visualizaion

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
# ## Set configs

# %%
root_dir = Path('/root/TB')

# %%
prj_root_dir = root_dir/ 'ref_addition' / 'gwas_test'
prj_root_dir.mkdir(exist_ok=True)
prj_root_dir

# %% [markdown]
# ### File path

# %%
file_path_file = prj_root_dir / 'file_path.csv'
file_path_df = pd.read_csv(file_path_file, index_col=0)
file_path_df

# %% [markdown]
# ### Drug list

# %%
# drugs list
drug_list = ['RIF', 'SM', 'LEV', 'EMB', 'RBT', 'INH', 'ETA', 'BDQ', 'DLM']
drug_list

# %% [markdown]
# ### GWAS result files

# %%
gwas_result_df_list = [pd.read_csv(f) for f in file_path_df['annotated_res']]

# %%
gwas_result_df_list[0]

# %%
chr_set = set()
for df in gwas_result_df_list:
    for c in df['chr'].unique():
        chr_set.add(c)

# %%
chr_set

# %% [markdown]
# ## 1. Labeling

# %%
def labeling_gwas_res(row):
    locus_tag = row['locus_tag']
    if type(locus_tag) == str:
        on_transcript = True
    else:
        on_transcript = not (np.isnan(locus_tag))
    # is_protein_coding = row['is_protein_coding']

    # if on_transcript:
    #     if is_protein_coding:
    #         label = 'protein_coding_on_transcript'
    #     else:
    #         label = 'non_protein_coding_on_transcript'
    # else:
    #     if is_protein_coding:
    #         raise Exception("error")
    #     else:
    #         label = 'not_on_transcript'
    
    if row['chr'] == 'NC_000962.3':
        is_h37rv = True
    else:
        is_h37rv = False
    
    return is_h37rv, on_transcript

# %%
for gwas_res_df in gwas_result_df_list:
    gwas_res_df[
        ['is_h37rv', 'on_transcript']
    ] = gwas_res_df.apply(labeling_gwas_res, axis=1).apply(pd.Series)

# %% [markdown]
# # 2. Manhattan Plot

# %%
import plotly.express as px
import plotly.graph_objects as go

# %% [markdown]
# ### (1) Function to figure plot

# %%
# select colors
fig = px.colors.qualitative.swatches()
# fig.show("png")

# %%
def manhattan_plot_gwas_res(gwas_res_df, drug):
    logpv_cutoff = 4
    
    max_position = gwas_res_df['accum_ps'].max()
    max_minus_logp = gwas_res_df['-logp_v'].max()
    
    # Build figure
    fig = go.Figure()
    
    
    legend_name_list = [
        'h37rv/on_trans',
        'h37rv/not_on_transcipt',
        'addtional/on_trans',
        'addtional/not_on_transcipt',
    ]
    legend_name_list.reverse() # make to stack
    
    color_list = [
        px.colors.qualitative.Pastel[0], # h37rv
        px.colors.qualitative.Pastel[1], # altseq
    ]
    df_list = [
        gwas_res_df[gwas_res_df['is_h37rv']], # h37rv
        gwas_res_df[~gwas_res_df['is_h37rv']], # altseq
    ]
    
    
    for color, df in zip(color_list, df_list):
        symbol_list = [
            'circle',
            'cross',
        ]
        df2_list = [
            df[df['on_transcript']],
            df[~df['on_transcript']]
        ]
        
        for symbol, df2 in zip(symbol_list, df2_list):
            name = legend_name_list.pop()
            
            fig.add_trace(
                go.Scatter(
                    name = name,
                    x=df2['accum_ps'],
                    y=df2['-logp_v'],
                    mode='markers',
                    marker_symbol = symbol,
                    marker_color=color,
                    marker_line_width = 1,
                    marker_size = 7,
                    showlegend=True,
                )
            )
    
    # text
    df = gwas_res_df[
        (gwas_res_df['-logp_v'] >=  logpv_cutoff)
    ]
    
    text=(
        df['locus_tag'].fillna('NA') + ';'
        + df['gene_name'].fillna('NA')
    )
    
    fig.add_trace(
        go.Scatter(
            name='GeneInfoOverCutoff',
            x=df['accum_ps'],
            y=df['-logp_v'] - max_minus_logp/100,
            mode='text',
            text=text,
            # textposition="bottom right",
            textposition="bottom center",
            textfont_size=12,
            showlegend=True,
        )
    )
    
    fig.update_layout(
        width= 1000,
        height= 500,
        title=f'Manhattan Plot - {drug}',
        template="plotly_white",
    )
    
    fig.add_hline(y=logpv_cutoff, line_width=2, line_dash="dash")
    fig.update_xaxes(title='Position', range=[0, max_position])
    fig.update_yaxes(title='-log(p)', range=[0, max_minus_logp+1])
    
    return fig

# %%
# test
idx = 1

fig = manhattan_plot_gwas_res(gwas_result_df_list[idx], drug_list[idx])

# %%
fig.show()

# %%
# test
idx = 2

fig = manhattan_plot_gwas_res(gwas_result_df_list[idx], drug_list[idx])

# %%
fig.show()

# %%
# test
idx = 5

fig = manhattan_plot_gwas_res(gwas_result_df_list[idx], drug_list[idx])

# %%
fig.show("png", scale=2)

# %%
# test
idx = 6

fig = manhattan_plot_gwas_res(gwas_result_df_list[idx], drug_list[idx])

# %%
fig.show("png", scale=2)

# %%
# test
idx = 7

fig = manhattan_plot_gwas_res(gwas_result_df_list[idx], drug_list[idx])

# %%
fig.show("png", scale=2)

# %%
# test
idx = 8

fig = manhattan_plot_gwas_res(gwas_result_df_list[idx], drug_list[idx])

# %%
fig.show("png", scale=2)

# %%
chrom_set = set()
for df in gwas_result_df_list:
    for v in df['chr'].unique():
        chrom_set.add(v)

# %%
chrom_set

# %% [markdown]
# ### (2) get figures and save

# %%
images_dir = prj_root_dir / 'images'
images_dir.mkdir(parents=True, exist_ok=True)
images_dir

# %%
output_image_file_list = [
    images_dir / f'manhat_{drug}.svg'
    for drug in drug_list
]

# %%
for drug, gwas_res_df, output_file in zip(drug_list, gwas_result_df_list, output_image_file_list):
    fig = manhattan_plot_gwas_res(gwas_res_df, drug)
    
    fig.write_image(str(output_file), scale=2)
