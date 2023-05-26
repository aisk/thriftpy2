# -*- coding: utf-8 -*-

from __future__ import absolute_import

from thriftpy2.transport import TTransportBase, TTransportException


async def readall(read_fn, sz):
    buff = b''
    have = 0
    while have < sz:
        chunk = await read_fn(sz - have)
        have += len(chunk)
        buff += chunk

        if len(chunk) == 0:
            raise TTransportException(
                TTransportException.END_OF_FILE,
                "End of file reading from transport",
            )

    return buff


class TAsyncTransportBase(TTransportBase):
    """Base class for Thrift async transport layer."""

    def is_open(self):
        raise NotImplementedError

    async def open(self):
        raise NotImplementedError

    def close(self):
        raise NotImplementedError

    async def _read(self, sz):
        raise NotImplementedError

    async def read(self, sz):
        return await readall(self._read, sz)

    def write(self, buf):
        raise NotImplementedError

    async def flush(self):
        raise NotImplementedError
