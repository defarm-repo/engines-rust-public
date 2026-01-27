import axios from 'axios';
import { config } from './config';

export const api = axios.create({
  baseURL: process.env.DEFARM_API_URL || 'https://connect.defarm.net',
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add auth interceptor
api.interceptors.request.use((config) => {
  const token = process.env.DEFARM_TOKEN || global.config?.get('token');
  const apiKey = process.env.DEFARM_API_KEY;

  if (apiKey) {
    config.headers['X-API-Key'] = apiKey;
  } else if (token) {
    config.headers['Authorization'] = `Bearer ${token}`;
  }

  return config;
});

// Error interceptor
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      console.error('Authentication failed. Please login again: defarm login');
      process.exit(1);
    }
    return Promise.reject(error);
  }
);
