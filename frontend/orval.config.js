module.exports = {
  jobsearch: {
    input: {
      target: 'http://localhost:8080/api/openapi.json',
    },
    output: {
      target: './src/generated/orval',
      client: 'fetch',
      mode: 'split',
    },
  },
};
