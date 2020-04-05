| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `.\target\release\sdr-heatmap.exe .\samples\0.csv` | 396.0 ± 22.2 | 378.8 | 444.5 | 1.00 |
| `python .\samples\heatmap.py .\samples\0.csv .\samples\0.csv-py.png` | 4839.4 ± 304.7 | 4619.9 | 5441.4 | 12.22 ± 1.03 |
| `.\samples\gopow.exe -i .\samples\0.csv -o .\samples\0.csv-go.png` | 2043.2 ± 62.8 | 2005.5 | 2219.6 | 5.16 ± 0.33 |
| `.\target\release\sdr-heatmap.exe .\samples\1-huge.csv` | 18836.9 ± 241.0 | 18693.9 | 19488.8 | 47.57 ± 2.73 |
| `python .\samples\heatmap.py .\samples\1-huge.csv .\samples\1-huge.csv-py.png` | 268466.3 ± 11084.9 | 264021.5 | 299911.2 | 677.93 ± 47.17 |
| `.\samples\gopow.exe -i .\samples\1-huge.csv -o .\samples\1-huge.csv-go.png` | 141555.7 ± 406.1 | 140898.7 | 142310.5 | 357.46 ± 20.04 |
