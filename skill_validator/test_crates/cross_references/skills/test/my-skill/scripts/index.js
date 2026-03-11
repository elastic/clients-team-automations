import { createClient } from '../../../shared/shared-skill/es-client.js';
import { helper } from './helper.js';
const config = require('../../../shared/shared-skill/config.json');

export async function run() {
    const client = createClient();
    return helper(client);
}
