const { Client: Client } = require('@elastic/elasticsearch')

const client = new Client({
  cloud: { id: process.env.ES_CLOUD_ID },
  auth: { apiKey: process.env.ES_API_KEY }
})

const payload = JSON.parse(process.env.PAYLOAD)

client.index({
  index: 'issues',
  id: payload.id,
  document: payload
})
