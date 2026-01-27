import chalk from 'chalk';
import ora from 'ora';
import Table from 'cli-table3';
import { api } from '../utils/api';
import { formatDate, truncate } from '../utils/format';

export const circuitsCommand = {
  async list() {
    try {
      const spinner = ora('Fetching circuits...').start();

      const response = await api.get('/api/circuits');

      spinner.stop();

      const circuits = response.data;

      if (!circuits || circuits.length === 0) {
        console.log(chalk.yellow('No circuits found'));
        console.log(chalk.gray('\nCreate a new circuit:'));
        console.log(chalk.gray('  $ defarm circuits create "My Circuit"'));
        return;
      }

      const table = new Table({
        head: [
          chalk.cyan('Name'),
          chalk.cyan('Members'),
          chalk.cyan('Public'),
          chalk.cyan('Created'),
        ],
        colWidths: [40, 15, 10, 20],
      });

      circuits.forEach((circuit: any) => {
        table.push([
          truncate(circuit.name, 38),
          circuit.members?.length || 0,
          circuit.permissions?.allow_public_visibility ? chalk.green('Yes') : chalk.gray('No'),
          formatDate(circuit.created_timestamp),
        ]);
      });

      console.log(table.toString());
      console.log(chalk.gray(`\nShowing ${circuits.length} circuits`));
    } catch (error: any) {
      console.error(chalk.red('Error fetching circuits:'), error.message);
      process.exit(1);
    }
  },

  async create(name: string, options: any) {
    try {
      const spinner = ora('Creating circuit...').start();

      const data: any = {
        name,
        description: options.description || '',
        permissions: {
          require_approval_for_push: false,
          require_approval_for_pull: false,
          allow_public_visibility: options.public || false,
        },
      };

      if (options.adapter) {
        data.adapter_config = {
          adapter_type: options.adapter,
          requires_approval: false,
          auto_migrate_existing: false,
          sponsor_adapter_access: true,
        };
      }

      const response = await api.post('/api/circuits', data);

      spinner.succeed(chalk.green('Circuit created successfully!'));

      const circuit = response.data.data;
      console.log(chalk.cyan('Circuit ID:'), circuit.circuit_id);
      console.log(chalk.cyan('Name:'), circuit.name);
      if (options.adapter) {
        console.log(chalk.cyan('Adapter:'), options.adapter);
      }
    } catch (error: any) {
      console.error(chalk.red('Error creating circuit:'), error.response?.data?.message || error.message);
      process.exit(1);
    }
  },

  async get(id: string) {
    try {
      const spinner = ora('Fetching circuit...').start();

      const response = await api.get(`/api/circuits/${id}`);

      spinner.stop();

      const circuit = response.data.data;

      console.log(chalk.green('\nCircuit Details:'));
      console.log(chalk.cyan('ID:'), circuit.circuit_id);
      console.log(chalk.cyan('Name:'), circuit.name);
      console.log(chalk.cyan('Description:'), circuit.description || 'None');
      console.log(chalk.cyan('Owner:'), circuit.owner_id);
      console.log(chalk.cyan('Status:'), circuit.status);
      console.log(chalk.cyan('Members:'), circuit.members?.length || 0);
      console.log(chalk.cyan('Public:'), circuit.permissions?.allow_public_visibility ? 'Yes' : 'No');
      console.log(chalk.cyan('Created:'), formatDate(circuit.created_timestamp));

      if (circuit.adapter_config) {
        console.log(chalk.cyan('\nBlockchain Adapter:'), circuit.adapter_config.adapter_type);
      }
    } catch (error: any) {
      console.error(chalk.red('Error fetching circuit:'), error.message);
      process.exit(1);
    }
  },

  async push(circuitId: string, localId: string) {
    try {
      const spinner = ora('Pushing item to circuit...').start();

      const response = await api.post(`/api/circuits/${circuitId}/push-local`, {
        local_id: localId,
      });

      spinner.succeed(chalk.green('Item pushed successfully!'));

      const result = response.data.data;
      console.log(chalk.cyan('DFID:'), result.dfid);

      if (result.storage) {
        console.log(chalk.cyan('Storage:'), result.storage.adapter_type);
        if (result.storage.cid) {
          console.log(chalk.cyan('IPFS CID:'), result.storage.cid);
        }
        if (result.storage.transaction_hash) {
          console.log(chalk.cyan('TX Hash:'), result.storage.transaction_hash);
        }
      }

      console.log(chalk.gray('\nItem is now globally tokenized!'));
    } catch (error: any) {
      console.error(chalk.red('Error pushing item:'), error.response?.data?.message || error.message);
      process.exit(1);
    }
  },

  async items(circuitId: string) {
    try {
      const spinner = ora('Fetching circuit items...').start();

      const response = await api.get(`/api/circuits/${circuitId}/items`);

      spinner.stop();

      const items = response.data.data;

      if (!items || items.length === 0) {
        console.log(chalk.yellow('No items in this circuit'));
        return;
      }

      const table = new Table({
        head: [chalk.cyan('DFID'), chalk.cyan('Identifiers'), chalk.cyan('Pushed At')],
        colWidths: [30, 50, 20],
      });

      items.forEach((item: any) => {
        const identifiers = item.identifiers
          ?.map((id: any) => `${id.key}:${id.value}`)
          .join(', ') || 'None';

        table.push([
          truncate(item.dfid, 28),
          truncate(identifiers, 48),
          formatDate(item.pushed_at || item.creation_timestamp),
        ]);
      });

      console.log(table.toString());
      console.log(chalk.gray(`\nTotal items: ${items.length}`));
    } catch (error: any) {
      console.error(chalk.red('Error fetching items:'), error.message);
      process.exit(1);
    }
  },

  async members(circuitId: string) {
    try {
      const spinner = ora('Fetching circuit members...').start();

      const response = await api.get(`/api/circuits/${circuitId}`);

      spinner.stop();

      const circuit = response.data.data;
      const members = circuit.members || [];

      if (members.length === 0) {
        console.log(chalk.yellow('No members in this circuit'));
        return;
      }

      const table = new Table({
        head: [chalk.cyan('Member ID'), chalk.cyan('Role'), chalk.cyan('Joined')],
        colWidths: [40, 15, 20],
      });

      members.forEach((member: any) => {
        table.push([
          truncate(member.member_id, 38),
          member.role === 'Owner' ? chalk.green(member.role) : member.role,
          formatDate(member.joined_timestamp),
        ]);
      });

      console.log(table.toString());
      console.log(chalk.gray(`\nTotal members: ${members.length}`));
    } catch (error: any) {
      console.error(chalk.red('Error fetching members:'), error.message);
      process.exit(1);
    }
  },
};
