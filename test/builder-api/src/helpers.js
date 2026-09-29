// Users we can authenticate as
global.boboBearer = 'Bearer bobo';
global.mystiqueBearer = 'Bearer mystique';
global.hankBearer = 'Bearer hank';
global.weskerBearer = 'Bearer wesker';
global.lkennedyBearer = 'Bearer lkennedy';
// accounts.name of boboBearer
global.boboAccountName = 'bobo';

// Use the same defaults as Builder's MemcacheCfg. Integration tests must connect
// to the same memcached instance as Builder to seed and inspect package entries.
const net = require('net');

function memcachedCommand(command, readResponse) {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({
      host: process.env.MEMCACHED_HOST || 'localhost',
      port: Number(process.env.MEMCACHED_PORT || 11211)
    });
    let response = Buffer.alloc(0);
    let completed = false;

    function finish(error, value) {
      if (completed) return;
      completed = true;
      socket.destroy();
      if (error) reject(error);
      else resolve(value);
    }

    socket.setTimeout(2000, () => finish(new Error('Memcached test command timed out')));
    socket.once('error', error => finish(error));
    socket.once('end', () => finish(new Error('Memcached closed an incomplete response')));
    socket.once('connect', () => socket.write(command));
    socket.on('data', chunk => {
      response = Buffer.concat([response, chunk]);
      try {
        const result = readResponse(response);
        if (result !== undefined) finish(null, result);
      } catch (error) {
        finish(error);
      }
    });
  });
}

function getMemcachedValue(key) {
  return memcachedCommand(`get ${key}\r\n`, response => {
    const headerEnd = response.indexOf('\r\n');
    if (headerEnd === -1) return;
    const header = response.slice(0, headerEnd).toString();
    if (header === 'END') return null;
    const match = /^VALUE \S+ \d+ (\d+)$/.exec(header);
    if (!match) throw new Error(`Unexpected memcached response: ${header}`);
    const valueStart = headerEnd + 2;
    const valueEnd = valueStart + Number(match[1]);
    if (response.length < valueEnd + 7) return;
    if (response.slice(valueEnd).toString() !== '\r\nEND\r\n') {
      throw new Error('Invalid memcached value terminator');
    }
    return response.slice(valueStart, valueEnd).toString();
  });
}

function setMemcachedValue(key, value) {
  // Flag 0 represents a string, as used by the Rust memcache client for JSON.
  const command = `set ${key} 0 60 ${Buffer.byteLength(value)}\r\n${value}\r\n`;
  return memcachedCommand(command, response => {
    if (response.indexOf('\r\n') === -1) return;
    if (response.toString() !== 'STORED\r\n') {
      throw new Error(`Unable to seed memcached: ${response.toString().trim()}`);
    }
    return true;
  });
}

module.exports = { getMemcachedValue, setMemcachedValue };
