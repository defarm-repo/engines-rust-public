import dayjs from 'dayjs';

export function formatDate(timestamp: number | string): string {
  if (!timestamp) return 'N/A';

  // Handle Unix timestamp (seconds)
  if (typeof timestamp === 'number') {
    return dayjs.unix(timestamp).format('YYYY-MM-DD HH:mm');
  }

  // Handle ISO string
  return dayjs(timestamp).format('YYYY-MM-DD HH:mm');
}

export function truncate(str: string, maxLength: number): string {
  if (!str) return '';
  if (str.length <= maxLength) return str;
  return str.substring(0, maxLength - 3) + '...';
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';

  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}
