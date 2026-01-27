import chalk from 'chalk';
import ora from 'ora';
import Table from 'cli-table3';
import { api } from '../utils/api';
import { formatDate, truncate } from '../utils/format';

export const itemsCommand = {
  async list(options: any) {
    try {
      const spinner = ora('Fetching items...').start();

      const response = await api.get('/api/items', {
        params: {
          limit: options.limit,
          offset: options.offset,
        },
      });

      spinner.stop();

      const items = response.data;

      if (!items || items.length === 0) {
        console.log(chalk.yellow('No items found'));
        return;
      }

      const table = new Table({
        head: [
          chalk.cyan('DFID/LID'),
          chalk.cyan('Identifiers'),
          chalk.cyan('Created'),
          chalk.cyan('Status'),
        ],
        colWidths: [30, 40, 20, 15],
      });

      items.forEach((item: any) => {
        const identifiers = item.identifiers
          .map((id: any) => `${id.key}:${id.value}`)
          .join(', ');

        table.push([
          truncate(item.dfid, 28),
          truncate(identifiers || 'None', 38),
          formatDate(item.creation_timestamp),
          item.status === 'Active' ? chalk.green(item.status) : chalk.gray(item.status),
        ]);
      });

      console.log(table.toString());
      console.log(chalk.gray(`\nShowing ${items.length} items`));
    } catch (error: any) {
      console.error(chalk.red('Error fetching items:'), error.message);
      process.exit(1);
    }
  },

  async create(options: any) {
    try {
      if (!options.key || !options.value) {
        console.error(chalk.red('Error: --key and --value are required'));
        process.exit(1);
      }

      const spinner = ora('Creating local item...').start();

      const enrichedData = options.data ? JSON.parse(options.data) : {};

      const response = await api.post('/api/items/local', {
        identifiers: [
          {
            namespace: options.namespace,
            key: options.key,
            value: options.value,
            id_type: 'Contextual',
            verified: false,
          },
        ],
        enriched_data: enrichedData,
      });

      spinner.succeed(chalk.green('Item created successfully!'));

      const item = response.data.data;
      console.log(chalk.cyan('Local ID:'), item.local_id);
      console.log(chalk.gray('Status: Pending tokenization'));
      console.log(chalk.gray('\nNext step: Push to circuit to get DFID'));
      console.log(chalk.gray(`  $ defarm circuits push <circuit-id> ${item.local_id}`));
    } catch (error: any) {
      console.error(chalk.red('Error creating item:'), error.message);
      process.exit(1);
    }
  },

  async get(dfid: string) {
    try {
      const spinner = ora('Fetching item...').start();

      const response = await api.get(`/api/items/${dfid}`);

      spinner.stop();

      const item = response.data;

      console.log(chalk.green('\nItem Details:'));
      console.log(chalk.cyan('DFID:'), item.dfid);
      console.log(chalk.cyan('Status:'), item.status);
      console.log(chalk.cyan('Created:'), formatDate(item.creation_timestamp));

      if (item.identifiers && item.identifiers.length > 0) {
        console.log(chalk.cyan('\nIdentifiers:'));
        item.identifiers.forEach((id: any) => {
          console.log(`  • ${id.namespace}:${id.key} = ${id.value} (${id.id_type})`);
        });
      }

      if (item.enriched_data && Object.keys(item.enriched_data).length > 0) {
        console.log(chalk.cyan('\nEnriched Data:'));
        console.log(JSON.stringify(item.enriched_data, null, 2));
      }
    } catch (error: any) {
      console.error(chalk.red('Error fetching item:'), error.message);
      process.exit(1);
    }
  },

  async timeline(dfid: string) {
    try {
      const spinner = ora('Fetching timeline...').start();

      const response = await api.get(`/api/timeline/items/${dfid}`);

      spinner.stop();

      const timeline = response.data.data.timeline;

      if (!timeline || timeline.length === 0) {
        console.log(chalk.yellow('No timeline entries found'));
        return;
      }

      console.log(chalk.green(`\nTimeline for ${dfid}:`));
      console.log(chalk.gray('─'.repeat(80)));

      timeline.forEach((entry: any, index: number) => {
        console.log(`\n${index + 1}. ${chalk.cyan(entry.entry_type)} - ${formatDate(entry.timestamp)}`);

        if (entry.event_type) {
          console.log(`   Event Type: ${chalk.yellow(entry.event_type)}`);
        }
        if (entry.operation) {
          console.log(`   Operation: ${chalk.yellow(entry.operation)}`);
        }
        if (entry.metadata) {
          console.log(`   Metadata: ${JSON.stringify(entry.metadata)}`);
        }
      });

      console.log(chalk.gray('\n─'.repeat(80)));
      console.log(chalk.gray(`Total entries: ${timeline.length}`));
    } catch (error: any) {
      console.error(chalk.red('Error fetching timeline:'), error.message);
      process.exit(1);
    }
  },

  async storage(dfid: string) {
    try {
      const spinner = ora('Fetching storage history...').start();

      const response = await api.get(`/api/items/${dfid}/storage-history`);

      spinner.stop();

      const storage = response.data.data;

      if (!storage || storage.length === 0) {
        console.log(chalk.yellow('No storage history found'));
        return;
      }

      console.log(chalk.green(`\nStorage History for ${dfid}:`));

      storage.forEach((entry: any, index: number) => {
        console.log(`\n${index + 1}. ${chalk.cyan(entry.adapter_type)}`);
        console.log(`   Location: ${entry.location}`);
        if (entry.cid) {
          console.log(`   IPFS CID: ${entry.cid}`);
        }
        if (entry.transaction_hash) {
          console.log(`   TX Hash: ${entry.transaction_hash}`);
        }
        console.log(`   Stored: ${formatDate(entry.stored_at)}`);
      });

      console.log(chalk.gray(`\nTotal storage locations: ${storage.length}`));
    } catch (error: any) {
      console.error(chalk.red('Error fetching storage:'), error.message);
      process.exit(1);
    }
  },
};
