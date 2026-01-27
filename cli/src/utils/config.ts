import Conf from 'conf';

export const config = new Conf({
  projectName: 'defarm-cli',
  defaults: {
    api_url: 'https://connect.defarm.net',
  },
});

declare global {
  var config: typeof import('conf').default;
}

global.config = config;
