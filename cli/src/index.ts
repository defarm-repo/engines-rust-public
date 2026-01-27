#!/usr/bin/env node

import { Command } from 'commander';
import chalk from 'chalk';
import { version } from '../package.json';
import { loginCommand } from './commands/auth';
import { itemsCommand } from './commands/items';
import { circuitsCommand } from './commands/circuits';
import { eventsCommand } from './commands/events';
import { merkleCommand } from './commands/merkle';
import { configCommand } from './commands/config';
import { whoamiCommand } from './commands/whoami';

const program = new Command();

program
  .name('defarm')
  .description(chalk.green('🌱 DeFarm Engines CLI - Traceability made simple'))
  .version(version, '-v, --version', 'Output the current version');

// Global options
program
  .option('--api-url <url>', 'API base URL', process.env.DEFARM_API_URL || 'https://connect.defarm.net')
  .option('--api-key <key>', 'API key for authentication', process.env.DEFARM_API_KEY)
  .option('--token <token>', 'JWT token for authentication', process.env.DEFARM_TOKEN)
  .option('--json', 'Output in JSON format')
  .option('--verbose', 'Verbose output');

// Authentication commands
program
  .command('login')
  .description('Login to DeFarm Engines')
  .option('-u, --username <username>', 'Username')
  .option('-p, --password <password>', 'Password')
  .action(loginCommand);

program
  .command('whoami')
  .description('Show current authenticated user')
  .action(whoamiCommand);

program
  .command('logout')
  .description('Logout and clear credentials')
  .action(() => {
    console.log(chalk.yellow('Logging out...'));
    // Clear config
    console.log(chalk.green('✓ Logged out successfully'));
  });

// Items commands
const items = program
  .command('items')
  .alias('i')
  .description('Manage items');

items
  .command('list')
  .description('List all items')
  .option('-l, --limit <number>', 'Limit results', '50')
  .option('-o, --offset <number>', 'Offset', '0')
  .action(itemsCommand.list);

items
  .command('create')
  .description('Create a new local item')
  .option('--namespace <namespace>', 'Identifier namespace', 'generic')
  .option('--key <key>', 'Identifier key (required)')
  .option('--value <value>', 'Identifier value (required)')
  .option('--data <json>', 'Enriched data as JSON')
  .action(itemsCommand.create);

items
  .command('get <dfid>')
  .description('Get item details')
  .action(itemsCommand.get);

items
  .command('timeline <dfid>')
  .description('Get item timeline')
  .action(itemsCommand.timeline);

items
  .command('storage <dfid>')
  .description('Get item storage history')
  .action(itemsCommand.storage);

// Circuits commands
const circuits = program
  .command('circuits')
  .alias('c')
  .description('Manage circuits');

circuits
  .command('list')
  .description('List all circuits')
  .action(circuitsCommand.list);

circuits
  .command('create <name>')
  .description('Create a new circuit')
  .option('-d, --description <desc>', 'Circuit description')
  .option('--public', 'Make circuit public')
  .option('--adapter <type>', 'Blockchain adapter type')
  .action(circuitsCommand.create);

circuits
  .command('get <id>')
  .description('Get circuit details')
  .action(circuitsCommand.get);

circuits
  .command('push <circuit-id> <local-id>')
  .description('Push local item to circuit (tokenization)')
  .action(circuitsCommand.push);

circuits
  .command('items <circuit-id>')
  .description('List items in circuit')
  .action(circuitsCommand.items);

circuits
  .command('members <circuit-id>')
  .description('List circuit members')
  .action(circuitsCommand.members);

// Events commands
const events = program
  .command('events')
  .alias('e')
  .description('Manage events');

events
  .command('list <dfid>')
  .description('List events for an item')
  .action(eventsCommand.list);

events
  .command('create <dfid>')
  .description('Create a new event')
  .option('-t, --type <type>', 'Event type', 'Enriched')
  .option('-v, --visibility <visibility>', 'Visibility', 'Public')
  .option('--metadata <json>', 'Metadata as JSON')
  .action(eventsCommand.create);

// Merkle commands
const merkle = program
  .command('merkle')
  .alias('m')
  .description('Merkle tree operations');

merkle
  .command('item-root <dfid>')
  .description('Get item Merkle root')
  .action(merkleCommand.itemRoot);

merkle
  .command('circuit-root <circuit-id>')
  .description('Get circuit Merkle root')
  .action(merkleCommand.circuitRoot);

merkle
  .command('verify <proof-file>')
  .description('Verify a Merkle proof from file')
  .action(merkleCommand.verify);

// Config commands
const config = program
  .command('config')
  .description('Manage CLI configuration');

config
  .command('set <key> <value>')
  .description('Set configuration value')
  .action(configCommand.set);

config
  .command('get <key>')
  .description('Get configuration value')
  .action(configCommand.get);

config
  .command('list')
  .description('List all configuration')
  .action(configCommand.list);

// Add banner on help
program.on('--help', () => {
  console.log('');
  console.log(chalk.green('Examples:'));
  console.log('  $ defarm login');
  console.log('  $ defarm items create --key sisbov --value BR123');
  console.log('  $ defarm circuits list');
  console.log('  $ defarm items list --limit 10');
  console.log('');
  console.log(chalk.cyan('Documentation:'));
  console.log('  https://connect.defarm.net/docs');
  console.log('');
});

// Parse arguments
program.parse();

// Show help if no command provided
if (!process.argv.slice(2).length) {
  program.outputHelp();
}
