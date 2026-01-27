import chalk from 'chalk';
import { config } from '../utils/config';

export const configCommand = {
  set(key: string, value: string) {
    config.set(key, value);
    console.log(chalk.green(`✓ Set ${key} = ${value}`));
  },

  get(key: string) {
    const value = config.get(key);
    if (value === undefined) {
      console.log(chalk.yellow(`Key "${key}" not found`));
    } else {
      console.log(chalk.cyan(key + ':'), value);
    }
  },

  list() {
    const all = config.store;
    console.log(chalk.green('Configuration:'));
    Object.entries(all).forEach(([key, value]) => {
      if (key === 'token') {
        console.log(chalk.cyan(key + ':'), chalk.gray(value as string));
      } else {
        console.log(chalk.cyan(key + ':'), value);
      }
    });
  },
};
