| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `.\target\release\sdr-heatmap.exe -vvvv .\samples\0-1row.csv` | 6.9 ± 0.5 | 6.2 | 10.7 | 1.00 |
| `python .\samples\heatmap.py .\samples\0-1row.csv .\samples\0-1row.csv-py.png` | 118.7 ± 3.0 | 115.5 | 126.9 | 17.19 ± 1.23 |
| `.\samples\gopow.exe -i .\samples\0-1row.csv -o .\samples\0-1row.csv-go.png` | 14.5 ± 0.7 | 13.7 | 19.8 | 2.10 ± 0.18 |
| `.\target\release\sdr-heatmap.exe -vvvv .\samples\0.csv` | 375.9 ± 1.6 | 373.8 | 379.7 | 54.41 ± 3.65 |
| `python .\samples\heatmap.py .\samples\0.csv .\samples\0.csv-py.png` | 4577.8 ± 14.3 | 4555.7 | 4600.4 | 662.58 ± 44.41 |
| `.\samples\gopow.exe -i .\samples\0.csv -o .\samples\0.csv-go.png` | 2004.2 ± 8.2 | 1995.2 | 2024.3 | 290.09 ± 19.46 |
