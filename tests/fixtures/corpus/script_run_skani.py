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
# # skani로 ANI 계산
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # ANI 계산 (skani dist)
#
# 대표서열 vs 모든 E. coli 게놈의 ANI를 계산합니다.
#
# **입력**:
# - `data/02-EcoliOrigin/01_download/genomes/*.fna` - 다운로드된 게놈
#
# **출력**:
# - `data/02-EcoliOrigin/03_cluster/skani_results.tsv` - 대표 vs 전체 ANI 결과

# %%
import pandas as pd
from pathlib import Path
import sys
import subprocess

sys.path.append('..')
from src.config import load_config, get_path

# %%
config = load_config()
genomes_dir = get_path('ecoli_genomes')
skani_dir = get_path('ecoli_skani')
cluster_dir = get_path('ecoli_cluster')
skani_dir.mkdir(parents=True, exist_ok=True)
cluster_dir.mkdir(parents=True, exist_ok=True)

rep_accession = config['test1_ecoli_origin']['representative_accession']
rep_genome = genomes_dir / f"{rep_accession}.fna"

print(f"Genomes directory: {genomes_dir}")
print(f"Cluster directory: {cluster_dir}")
print(f"Representative: {rep_accession}")
print(f"Representative path: {rep_genome}")

if not rep_genome.exists():
    raise FileNotFoundError(f"Representative genome not found: {rep_genome}")

# %%
genome_files = sorted(str(f) for f in genomes_dir.glob("*.fna"))
print(f"\nDownloaded genomes: {len(genome_files):,}")

# %% [markdown]
# ---
# ## skani dist 실행 (대표 vs 전체)

# %%
query_list_path = skani_dir / "query_genomes.txt"
query_list_path.write_text('\n'.join(genome_files))

output_path = cluster_dir / "skani_results.tsv"
SKANI = '/root/miniforge3/envs/skani/bin/skani'

if not output_path.exists():
    cmd = [
        SKANI, 'dist',
        '-r', str(rep_genome),
        '--ql', str(query_list_path),
        '-o', str(output_path),
        '-t', '16',
    ]

    print(f"\nRunning: {' '.join(cmd)}")
    print(f"Reference: {rep_accession}")
    print(f"Queries: {len(genome_files):,} genomes")

    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"ERROR: {result.stderr}")
    else:
        print("skani dist completed successfully")
else:
    print(f"Skipping - results already exist: {output_path}")

# %% [markdown]
# ---
# ## 결과 분석

# %%
print("\n" + "="*60)
print("결과 분석")
print("="*60)

if output_path.exists():
    results_df = pd.read_csv(output_path, sep='\t')
    print(f"Total results: {len(results_df):,}")

    # 컬럼 확인
    print(f"\nColumns: {results_df.columns.tolist()}")

    # ANI 분포
    print(f"\nANI Statistics:")
    print(f"  Min: {results_df['ANI'].min():.2f}%")
    print(f"  Max: {results_df['ANI'].max():.2f}%")
    print(f"  Mean: {results_df['ANI'].mean():.2f}%")
    print(f"  Median: {results_df['ANI'].median():.2f}%")

    # ANI 분포 히스토그램 (텍스트)
    print(f"\nANI Distribution:")
    bins = [90, 95, 98, 99, 99.5, 99.9, 100]
    for i in range(len(bins)-1):
        count = ((results_df['ANI'] >= bins[i]) & (results_df['ANI'] < bins[i+1])).sum()
        print(f"  {bins[i]:.1f}% - {bins[i+1]:.1f}%: {count:,}")
    count_100 = (results_df['ANI'] >= 99.9).sum()
    print(f"  >= 99.9%: {count_100:,}")

    # AF 분포
    for af_col in ['Align_fraction_ref', 'Align_fraction_query']:
        if af_col in results_df.columns:
            print(f"\n{af_col} Statistics:")
            print(f"  Min: {results_df[af_col].min():.4f}")
            print(f"  Max: {results_df[af_col].max():.4f}")
            print(f"  Mean: {results_df[af_col].mean():.4f}")
else:
    print("No results file found. Run skani first.")

# %% [markdown]
# ---
# ## 다음 단계
#
# ANI 계산이 완료되면 `02_03_filter_cluster.py`를 실행하여
# ANI ≥ 99.9% & AF ≥ 0.95 기준으로 필터링합니다.
