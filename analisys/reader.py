import mmap
import struct
import pyarrow as pa
from pathlib import Path
import polars as pl


def read_col(count, dtype, path):
    with open(path, 'rb') as col_file:
        mem = mmap.mmap(col_file.fileno(), length=count * 8, access=mmap.ACCESS_READ)
        buf = pa.py_buffer(mem)
        return pa.Array.from_buffers(dtype, count, [None, buf])


def read_port_table(port, name, data_dir):
    data_dir = Path(data_dir)
    with open(data_dir / f'{port}.count', 'rb') as f:
        count = struct.unpack('<Q', f.read())[0]
    ts_arr = read_col(count, pa.timestamp('ns'), data_dir / f'{port}.ts')
    val_arr = read_col(count, pa.uint64(), data_dir / f'{port}.val')
    return pl.from_arrow(
        pa.Table.from_arrays([ts_arr, val_arr], names=['ts', name])
    ).sort('ts')


def scale_df(data, ports_and_names, scale_ts):
    """
    Stretch time by the ``scale_ts`` factor.
    """
    if scale_ts == 1:
        return data
    int_arr = data['ts'].to_arrow().cast(pa.int64())
    int_pl = pl.from_arrow(int_arr).alias("ts")
    scaled_pl = ((int_pl - int_arr[0].as_py()) * scale_ts) + int_arr[0].as_py()
    scaled_ts_arr = scaled_pl.to_arrow().cast(pa.timestamp('ns'))
    names = ['ts'] + [name for name in ports_and_names.values()]
    cols = [scaled_ts_arr] + [data[name].to_arrow() for name in ports_and_names.values()]
    arr_table = pa.Table.from_arrays(cols, names=names)
    return pl.from_arrow(arr_table).sort('ts')


def read_ports_table(ports_and_names, data_dir='data', scale_ts=1):
    tables = [read_port_table(port, name, data_dir) for (port, name) in ports_and_names.items()]
    # concatenate all columns and merge the `ts` column
    data = pl.concat(tables, how='diagonal').fill_null(0).sort('ts')
    return scale_df(data, ports_and_names, scale_ts)
