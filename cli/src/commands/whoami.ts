import chalk from 'chalk';
import { config } from '../utils/config';

export async function whoamiCommand() {
  const userId = config.get('user_id');
  const workspaceId = config.get('workspace_id');
  const token = config.get('token');

  if (!userId || !token) {
    console.log(chalk.yellow('Not logged in'));
    console.log(chalk.gray('Run: defarm login'));
    return;
  }

  console.log(chalk.green('Authenticated as:'));
  console.log(chalk.cyan('User ID:'), userId);
  console.log(chalk.cyan('Workspace:'), workspaceId);
  console.log(chalk.cyan('Token:'), chalk.gray('***' + (token as string).slice(-8)));
}
