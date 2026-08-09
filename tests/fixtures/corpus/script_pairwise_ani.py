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
# # pairwise ANI 계산
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Pairwise ANI 계산
#
# ANI >= 98% 클러스터 내의 게놈들 간 pairwise ANI를 계산합니다.
# skani triangle 모드를 사용하여 all-vs-all ANI를 계산합니다.
#
# **입력**:
# - `data/results/ecoli_high_similarity_cluster.tsv` - 고유사도 클러스터
# - `data/raw/uhgg/genomes/ecoli/*.fna` - E. coli FASTA 파일들
#
# **출력**:
# - `data/results/ecoli_pairwise_ani.tsv` - Pairwise ANI 결과
# - `data/results/ecoli_pairwise_ani_heatmap.png` - ANI 히트맵
# - `data/logs/ecoli_analysis/skani_pairwise.log` - 실행 로그

# %%
import pandas as pd
from pathlib import Path
from typing import Tuple, List
import matplotlib.pyplot as plt
import seaborn as sns
import sys

sys.path.append('..')
from src import sem
from src.config import load_config, get_path

# 그래프 스타일 설정
sns.set_style("whitegrid")
plt.rcParams['figure.dpi'] = 100

# %%
config = load_config()
uhgg_dir = get_path('uhgg_data')
results_dir = get_path('results')

genomes_dir = uhgg_dir / "genomes" / "ecoli"

print(f"E. coli genomes directory: {genomes_dir}")
print(f"Results directory: {results_dir}")

# %%
cluster_path = results_dir / "ecoli_high_similarity_cluster.tsv"
print(f"\nLoading high similarity cluster from: {cluster_path}")

if not cluster_path.exists():
    raise FileNotFoundError(
        f"Cluster file not found: {cluster_path}\n"
        "Run 05_run_skani.py first to generate the cluster file."
    )

cluster_df = pd.read_csv(cluster_path, sep='\t')
print(f"Loaded {len(cluster_df):,} genomes in high similarity cluster")

# %%
print("\n" + "="*60)
print("게놈 파일 수집")
print("="*60)

# Query_file 컬럼에서 게놈 ID 추출
genome_files = []
missing_genomes = []

for _, row in cluster_df.iterrows():
    # Query_file 경로에서 genome_id 추출
    query_file = Path(row['Query_file'])
    genome_id = query_file.stem  # .fna 제거
    genome_path = genomes_dir / f"{genome_id}.fna"

    if genome_path.exists():
        genome_files.append(str(genome_path))
    else:
        missing_genomes.append(genome_id)

print(f"\nTotal genomes in cluster: {len(cluster_df):,}")
print(f"  Available: {len(genome_files):,}")
print(f"  Missing: {len(missing_genomes):,}")

if len(genome_files) < 2:
    raise ValueError("Need at least 2 genomes for pairwise comparison!")

# %% [markdown]
# ---
# ## skani triangle 실행

# %%
query_list_path = results_dir / "ecoli_pairwise_query_list.txt"
with open(query_list_path, 'w') as f:
    for genome_path in genome_files:
        f.write(f"{genome_path}\n")

print(f"\nQuery list saved to: {query_list_path}")
print(f"Total genomes for pairwise comparison: {len(genome_files):,}")

# %%
output_path = results_dir / "ecoli_pairwise_ani.tsv"

skani_cmd = f'''
source /opt/mambaforge/etc/profile.d/conda.sh
conda activate skani
skani triangle \\
    -l "{query_list_path}" \\
    -o "{output_path}" \\
    -t {config['execution']['max_threads']}
'''

print("\n" + "="*60)
print("skani triangle 명령어")
print("="*60)
print(f"Genomes: {len(genome_files):,}")
print(f"Threads: {config['execution']['max_threads']}")
print(f"Output: {output_path}")

# %%
print("\n" + "="*60)
print("sem으로 skani triangle 실행")
print("="*60)

log_dir = sem.init_group("ecoli_analysis")
print(f"Log directory: {log_dir}")

sem.add(
    command=skani_cmd,
    group="ecoli_analysis",
    label="skani_pairwise",
    parallel=1  # 단일 분석 작업
)

print("\nSubmitted skani triangle job")
print(f"Check log: cat {log_dir}/skani_pairwise.log")
print("Or in Python: sem.log('ecoli_analysis', 'skani_pairwise')")

# %%
# 작업이 완료될 때까지 대기
# sem.wait("ecoli_analysis")

# %%
def show_log(lines: int = 30):
    """skani 실행 로그 확인"""
    print(sem.tail("ecoli_analysis", "skani_pairwise", lines=lines))

# 사용: show_log()

# %% [markdown]
# ---
# ## 결과 분석 (skani 완료 후 실행)
#
# skani triangle 출력은 PHYLIP-style 하삼각 행렬 형식:
# - 1행: 게놈 수
# - 2행~: 게놈경로 [ANI1] [ANI2] ...

# %%
def parse_skani_triangle(file_path: Path) -> Tuple[List[str], pd.DataFrame]:
    """
    skani triangle 출력 파싱

    Returns:
        (genome_ids, pairwise_df) - 게놈 ID 리스트와 pairwise ANI DataFrame
    """
    with open(file_path, 'r') as f:
        lines = f.readlines()

    n_genomes = int(lines[0].strip())
    genome_ids = []
    ani_matrix = []

    for i, line in enumerate(lines[1:], start=0):
        parts = line.strip().split('\t')
        genome_path = parts[0]
        genome_id = Path(genome_path).stem
        genome_ids.append(genome_id)

        # ANI 값들 (하삼각 행렬)
        ani_values = [float(x) for x in parts[1:]] if len(parts) > 1 else []
        ani_matrix.append(ani_values)

    # pairwise DataFrame 생성
    pairs = []
    for i in range(len(genome_ids)):
        for j, ani in enumerate(ani_matrix[i]):
            pairs.append({
                'genome_i': genome_ids[j],
                'genome_j': genome_ids[i],
                'ANI': ani
            })

    pairwise_df = pd.DataFrame(pairs)
    return genome_ids, pairwise_df

# %%
if not output_path.exists():
    print(f"\nOutput file not found: {output_path}")
    print("Wait for skani to complete, then re-run this cell.")
else:
    print("\n" + "="*60)
    print("Pairwise ANI 결과 로드")
    print("="*60)

    genome_ids_parsed, pairwise_df = parse_skani_triangle(output_path)
    print(f"\nTotal genomes: {len(genome_ids_parsed):,}")
    print(f"Pairwise comparisons: {len(pairwise_df):,}")

# %%
if output_path.exists():
    print("\n" + "="*60)
    print("Pairwise ANI 통계")
    print("="*60)
    print(pairwise_df['ANI'].describe())

    print(f"\n최소 ANI: {pairwise_df['ANI'].min():.2f}%")
    print(f"최대 ANI: {pairwise_df['ANI'].max():.2f}%")
    print(f"평균 ANI: {pairwise_df['ANI'].mean():.2f}%")

# %%
if output_path.exists():
    n_genomes = len(genome_ids_parsed)

    if n_genomes <= 50:
        print("\n" + "="*60)
        print("ANI 히트맵 생성")
        print("="*60)

        # Pivot table 생성 (genome_i, genome_j, ANI 형식)
        pivot_df = pairwise_df.pivot(index='genome_i', columns='genome_j', values='ANI')

        # 대칭 행렬로 변환 (하삼각 → 전체)
        for i in pivot_df.index:
            for j in pivot_df.columns:
                if pd.isna(pivot_df.loc[i, j]) and j in pivot_df.index and i in pivot_df.columns:
                    if not pd.isna(pivot_df.loc[j, i]):
                        pivot_df.loc[i, j] = pivot_df.loc[j, i]
        # 대각선은 100%
        for i in pivot_df.index:
            if i in pivot_df.columns:
                pivot_df.loc[i, i] = 100.0

        # 히트맵
        fig, ax = plt.subplots(figsize=(12, 10))
        sns.heatmap(pivot_df, annot=True if n_genomes <= 20 else False,
                    fmt='.1f', cmap='RdYlGn', vmin=97, vmax=100,
                    ax=ax, square=True)
        ax.set_title('Pairwise ANI Heatmap (ANI >= 98% cluster)', fontsize=14, fontweight='bold')
        plt.tight_layout()

        heatmap_path = results_dir / 'ecoli_pairwise_ani_heatmap.png'
        plt.savefig(heatmap_path, dpi=300, bbox_inches='tight')
        print(f"\nSaved heatmap to: {heatmap_path}")

        plt.show()
    else:
        print(f"\n게놈 수가 {n_genomes}개로 너무 많아 히트맵 생성을 건너뜁니다.")
        print("대신 ANI 분포 히스토그램을 생성합니다.")

        fig, ax = plt.subplots(figsize=(10, 6))
        ax.hist(pairwise_df['ANI'], bins=50, edgecolor='black', alpha=0.7)
        ax.set_xlabel('ANI (%)', fontsize=12)
        ax.set_ylabel('Count', fontsize=12)
        ax.set_title('Pairwise ANI Distribution', fontsize=14, fontweight='bold')
        ax.grid(True, alpha=0.3)

        plt.tight_layout()

        hist_path = results_dir / 'ecoli_pairwise_ani_histogram.png'
        plt.savefig(hist_path, dpi=300, bbox_inches='tight')
        print(f"\nSaved histogram to: {hist_path}")

        plt.show()

# %%
if output_path.exists():
    print("\n" + "="*60)
    print("최종 요약")
    print("="*60)

    print(f"\n클러스터 내 게놈 수: {len(genome_ids_parsed):,}")
    print(f"Pairwise 비교 수: {len(pairwise_df):,}")
    print(f"ANI 범위: {pairwise_df['ANI'].min():.2f}% - {pairwise_df['ANI'].max():.2f}%")

    print(f"\n결과 파일: {output_path}")

# %% [markdown]
# ---
# ## 다음 단계
#
# Pairwise ANI 계산이 완료되면 `07_simulate_reads.py`를 실행하여
# Read 시뮬레이션을 수행합니다.
