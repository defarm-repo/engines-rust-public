import inquirer from 'inquirer';
import chalk from 'chalk';
import ora from 'ora';
import { api } from '../utils/api';
import { config } from '../utils/config';

export async function loginCommand(options: any) {
  try {
    let username = options.username;
    let password = options.password;

    // If not provided, prompt
    if (!username || !password) {
      const answers = await inquirer.prompt([
        {
          type: 'input',
          name: 'username',
          message: 'Username:',
          when: () => !username,
        },
        {
          type: 'password',
          name: 'password',
          message: 'Password:',
          mask: '*',
          when: () => !password,
        },
      ]);

      username = username || answers.username;
      password = password || answers.password;
    }

    const spinner = ora('Logging in...').start();

    const response = await api.post('/api/auth/login', {
      username,
      password,
    });

    if (response.data.success) {
      const { token, user_id, workspace_id } = response.data.data;

      // Save to config
      config.set('token', token);
      config.set('user_id', user_id);
      config.set('workspace_id', workspace_id);

      spinner.succeed(chalk.green('Login successful!'));
      console.log(chalk.gray(`User ID: ${user_id}`));
      console.log(chalk.gray(`Workspace: ${workspace_id}`));
    } else {
      spinner.fail(chalk.red('Login failed'));
    }
  } catch (error: any) {
    console.error(chalk.red('Login failed:'), error.response?.data?.message || error.message);
    process.exit(1);
  }
}
