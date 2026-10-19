// Which service a server, a plugin or a connector is for. The names are made up
// or public package names; nothing here reads an agent's configuration.

import { describe, expect, it } from 'vitest';
import { pluginName, serviceOf } from './brands';

describe('serviceOf', () => {
  it('finds a service by a whole word of the name', () => {
    expect(serviceOf('github')).toBe('github');
    expect(serviceOf('Notion')).toBe('notion');
    expect(serviceOf('figma-desktop')).toBe('figma');
    expect(serviceOf('claude.ai Gmail')).toBe('gmail');
    expect(serviceOf('my_linear_tools')).toBe('linear');
  });

  it('finds a service named by two words, with or without a dash', () => {
    expect(serviceOf('claude.ai Google Drive')).toBe('google-drive');
    expect(serviceOf('google-calendar')).toBe('google-calendar');
    expect(serviceOf('googledrive')).toBe('google-drive');
    expect(serviceOf('gdrive')).toBe('google-drive');
  });

  it('knows the other words a service goes by', () => {
    expect(serviceOf('jira')).toBe('atlassian');
    expect(serviceOf('postgres')).toBe('postgresql');
    expect(serviceOf('api.githubcopilot.com')).toBe('github');
  });

  it('falls back to the package, then the host', () => {
    expect(serviceOf('repo-tools', '@modelcontextprotocol/server-github', null)).toBe('github');
    expect(serviceOf('issues', null, 'mcp.linear.app')).toBe('linear');
    expect(serviceOf('db', 'supabase-mcp', 'mcp.vercel.com')).toBe('supabase');
  });

  it('prefers the name over the package and the host', () => {
    expect(serviceOf('notion', '@modelcontextprotocol/server-github', 'mcp.slack.com')).toBe('notion');
  });

  it('gives nothing for a row that names no known service', () => {
    expect(serviceOf('files', 'files-mcp', null)).toBeNull();
    expect(serviceOf('docs-search', null, 'docs.example.com')).toBeNull();
    expect(serviceOf(null, undefined, '')).toBeNull();
    expect(serviceOf()).toBeNull();
  });

  it('matches whole words only', () => {
    expect(serviceOf('canvas')).toBeNull();
    expect(serviceOf('slacker')).toBeNull();
    expect(serviceOf('nonlinear')).toBeNull();
  });
});

describe('pluginName', () => {
  it('leaves the marketplace out, so its name cannot name a service', () => {
    expect(pluginName('figma@acme-plugins')).toBe('figma');
    expect(serviceOf(pluginName('formatter@github-market'))).toBeNull();
    expect(pluginName('local-tool')).toBe('local-tool');
  });
});
