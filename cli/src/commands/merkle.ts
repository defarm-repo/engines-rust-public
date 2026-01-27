import chalk from 'chalk';
import ora from 'ora';
import { api } from '../utils/api';
import fs from 'fs';

export const merkleCommand = {
  async itemRoot(dfid: string) {
    try {
      const spinner = ora('Fetching item Merkle root...').start();

      const response = await api.get(`/api/merkle/items/${dfid}/merkle-root`);

      spinner.stop();

      const data = response.data.data;

      console.log(chalk.green('\nItem Merkle Root:'));
      console.log(chalk.cyan('DFID:'), data.dfid);
      console.log(chalk.cyan('Merkle Root:'), chalk.yellow(data.merkle_root));
      console.log(chalk.cyan('Event Count:'), data.event_count);
      console.log(chalk.cyan('Computed At:'), data.computed_at);
    } catch (error: any) {
      console.error(chalk.red('Error:'), error.message);
      process.exit(1);
    }
  },

  async circuitRoot(circuitId: string) {
    try {
      const spinner = ora('Fetching circuit Merkle root...').start();

      const response = await api.get(`/api/merkle/circuits/${circuitId}/merkle-root`);

      spinner.stop();

      const data = response.data.data;

      console.log(chalk.green('\nCircuit Merkle Root:'));
      console.log(chalk.cyan('Circuit ID:'), data.circuit_id);
      console.log(chalk.cyan('Merkle Root:'), chalk.yellow(data.merkle_root));
      console.log(chalk.cyan('Item Count:'), data.item_count);
      console.log(chalk.cyan('Computed At:'), data.computed_at);

      if (data.items && data.items.length > 0) {
        console.log(chalk.cyan('\nItems:'));
        data.items.forEach((item: any) => {
          console.log(`  • ${item.dfid} (${item.event_count} events)`);
        });
      }
    } catch (error: any) {
      console.error(chalk.red('Error:'), error.message);
      process.exit(1);
    }
  },

  async verify(proofFile: string) {
    try {
      const spinner = ora('Verifying Merkle proof...').start();

      const proof = JSON.parse(fs.readFileSync(proofFile, 'utf8'));

      const response = await api.post('/api/merkle/verify-proof', { proof });

      spinner.stop();

      const result = response.data.data;

      if (result.is_valid) {
        console.log(chalk.green('✓ Proof is VALID'));
        console.log(chalk.cyan('Verified At:'), result.verified_at);
      } else {
        console.log(chalk.red('✗ Proof is INVALID'));
      }
    } catch (error: any) {
      console.error(chalk.red('Error:'), error.message);
      process.exit(1);
    }
  },
};
