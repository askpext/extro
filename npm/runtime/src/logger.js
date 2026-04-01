/**
 * Extro Logger - Structured logging for debugging
 * 
 * Usage:
 *   import { logger } from './logger.js';
 *   logger.info('Message', { data });
 *   logger.error('Error', error);
 */

const LOG_PREFIX = '[Extro]';
const LOG_COLORS = {
  info: '#3b82f6',    // blue
  warn: '#f59e0b',    // amber
  error: '#ef4444',   // red
  debug: '#6b7280',   // gray
};

export const logger = {
  /**
   * Log info message
   */
  info(message, data) {
    this._log('info', message, data);
  },

  /**
   * Log warning message
   */
  warn(message, data) {
    this._log('warn', message, data);
  },

  /**
   * Log error message
   */
  error(message, error) {
    this._log('error', message, error);
  },

  /**
   * Log debug message (only in development)
   */
  debug(message, data) {
    if (globalThis.EXTRO_DEBUG) {
      this._log('debug', message, data);
    }
  },

  /**
   * Log command flow (for tracing)
   */
  command(command, surface, result) {
    console.log(`${LOG_PREFIX} 🔄 Command: ${command.action} from ${surface}`, {
      command,
      result,
      timestamp: new Date().toISOString(),
    });
  },

  /**
   * Internal log method
   */
  _log(level, message, data) {
    const color = LOG_COLORS[level];
    const timestamp = new Date().toISOString();
    const logData = {
      timestamp,
      level: level.toUpperCase(),
      message,
      ...(data ? { data } : {}),
    };

    console.log(
      `%c${LOG_PREFIX} ${level.toUpperCase()}`,
      `color: ${color}; font-weight: bold;`,
      logData
    );
  },
};

/**
 * Performance timer for measuring operation duration
 */
export function createTimer(label) {
  const start = performance.now();
  return {
    end(message) {
      const duration = performance.now() - start;
      logger.debug(`${label}: ${message}`, { duration: `${duration.toFixed(2)}ms` });
      return duration;
    },
  };
}

/**
 * Error handler with logging
 */
export function withErrorLog(fn, context) {
  return async (...args) => {
    try {
      return await fn(...args);
    } catch (error) {
      logger.error(`${context} failed`, error);
      throw error;
    }
  };
}
