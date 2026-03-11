export function helper(client) {
    return client.search({ index: 'test' });
}
