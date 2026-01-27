import chalk from 'chalk';
import ora from 'ora';
import Table from 'cli-table3';
import { api } from '../utils/api';
import { formatDate } from '../utils/format';

export const eventsCommand = {
  async list(dfid: string) {
    try {
      const spinner = ora('Fetching events...').start();

      const response = await api.get(`/api/events/item/${dfid}`);

      spinner.stop();

      const events = response.data;

      if (!events || events.length === 0) {
        console.log(chalk.yellow('No events found for this item'));
        return;
      }

      const table = new Table({
        head: [chalk.cyan('Type'), chalk.cyan('Source'), chalk.cyan('Visibility'), chalk.cyan('Date')],
        colWidths: [20, 30, 15, 20],
      });

      events.forEach((event: any) => {
        table.push([
          chalk.yellow(event.event_type),
          event.source,
          event.visibility === 'Public' ? chalk.green('Public') : chalk.gray('Private'),
          formatDate(event.timestamp),
        ]);
      });

      console.log(table.toString());
      console.log(chalk.gray(`\nTotal events: ${events.length}`));
    } catch (error: any) {
      console.error(chalk.red('Error fetching events:'), error.message);
      process.exit(1);
    }
  },

  async create(dfid: string, options: any) {
    try {
      const spinner = ora('Creating event...').start();

      const metadata = options.metadata ? JSON.parse(options.metadata) : {};

      const response = await api.post('/api/events', {
        dfid,
        event_type: options.type,
        visibility: options.visibility,
        metadata,
      });

      spinner.succeed(chalk.green('Event created successfully!'));

      const event = response.data.data;
      console.log(chalk.cyan('Event ID:'), event.event_id);
      console.log(chalk.cyan('Type:'), event.event_type);
      console.log(chalk.cyan('Visibility:'), event.visibility);
    } catch (error: any) {
      console.error(chalk.red('Error creating event:'), error.response?.data?.message || error.message);
      process.exit(1);
    }
  },
};
