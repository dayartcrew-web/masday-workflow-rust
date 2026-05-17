/**
 * ReActAgent - Reason + Act loop with tool use.
 *
 * Ported from agentic-llm-mem ReAct agent pattern.
 * Supports step-by-step reasoning with memory tool access.
 * Uses ILLMProvider from @mcp-rebuild/llm.
 */

import { createLogger } from '@mcp-rebuild/core';
import type { ILLMProvider, LLMOptions } from '@mcp-rebuild/llm';
import type {
  ToolDefinition,
  ReActTrace,
  ReActResult,
} from './types.js';

const logger = createLogger('intelligence:react');

// --- Default Tool Definitions ---

const MEMORY_TOOLS: ToolDefinition[] = [
  {
    name: 'search_memory',
    description: 'Search stored memories for relevant information. Use when unsure about user preferences, facts, or past context.',
    parameters: {
      query: { type: 'string', description: 'Search query', required: true },
      limit: { type: 'number', description: 'Max results (default 5)' },
    },
  },
  {
    name: 'add_memory',
    description: 'Save important information to memory. Use for user preferences, facts, decisions, or explicit "remember" requests.',
    parameters: {
      content: { type: 'string', description: 'Content to remember', required: true },
      type: { type: 'string', description: 'Memory type: fact|preference|skill|experience|strategy' },
      importance: { type: 'number', description: 'Importance 0-1 (default 0.5)' },
    },
  },
];

// --- Internal Types ---

interface ReActThought {
  thought: string;
  action: string | null;
  action_input: Record<string, unknown> | null;
  final_answer: string | null;
}

/** Provider interface for memory operations in the ReAct agent. */
export interface ReActMemoryProvider {
  search(query: string, options?: { limit?: number }): Promise<Array<{
    memory: { id: string; content: string; type: string; importance: number };
    score: number;
  }>>;
  add(content: string, options?: { type?: string; importance?: number; tags?: string[]; source?: string }): Promise<{ id: string; content: string; type: string; importance: number }>;
}

// --- Default System Prompt ---

const DEFAULT_SYSTEM_PROMPT = `You are an AI agent with persistent memory. You reason step-by-step and use tools when needed.

AVAILABLE TOOLS:
- search_memory: Search stored memories for relevant information.
  Params: {"query": "search query", "limit": 5}
- add_memory: Save important information to memory.
  Params: {"content": "content to save", "type": "fact|preference|skill|experience|strategy", "importance": 0.5}

You MUST respond in this exact JSON format:
{
  "thought": "your reasoning about what to do next",
  "action": "tool_name_or_null",
  "action_input": {"param": "value"} or null,
  "final_answer": "your final answer to the user, or null if you need more steps"
}

RULES:
- Always think before acting
- Search memory FIRST if you need context about the user
- Save IMPORTANT information (preferences, facts, decisions) to memory
- Do NOT save trivial information (greetings, questions, small talk)
- Use "final_answer" ONLY when you have enough information to respond
- Maximum 5 reasoning steps`;

// --- ReAct Agent ---

export class ReActAgent {
  private readonly llm: ILLMProvider;
  private readonly memory: ReActMemoryProvider;
  private readonly _tools: ToolDefinition[];
  private readonly maxSteps: number;
  private readonly temperature: number;
  private readonly systemPrompt: string;

  constructor(config: {
    llm: ILLMProvider;
    memory: ReActMemoryProvider;
    tools?: ToolDefinition[];
    maxSteps?: number;
    temperature?: number;
    systemPrompt?: string;
  }) {
    this.llm = config.llm;
    this.memory = config.memory;
    this._tools = config.tools ?? MEMORY_TOOLS;
    this.maxSteps = config.maxSteps ?? 5;
    this.temperature = config.temperature ?? 0.4;
    this.systemPrompt = config.systemPrompt ?? DEFAULT_SYSTEM_PROMPT;
  }

  /** Run the ReAct loop for a user input. */
  async run(userInput: string): Promise<ReActResult> {
    const traces: ReActTrace[] = [];
    let toolCalls = 0;
    let scratchpad = '';

    // Phase 1: Deterministic memory injection
    try {
      const injectedMemories = await this.memory.search(userInput, { limit: 3 });
      if (injectedMemories.length > 0) {
        const memContext = injectedMemories.map(m =>
          `[${m.memory.type}, importance: ${m.memory.importance}] ${m.memory.content}`
        ).join('\n');
        scratchpad = `Pre-loaded memories:\n${memContext}\n`;
      }
    } catch (error: unknown) {
      logger.warn({ error: error instanceof Error ? error.message : String(error) }, 'Memory injection failed, continuing without context');
    }

    // Phase 2: ReAct loop
    for (let step = 0; step < this.maxSteps; step++) {
      const prompt = `${this.systemPrompt}\n\nUser: ${userInput}\n\n${scratchpad}`;
      const options: LLMOptions = { temperature: this.temperature };
      const response = await this.llm.complete(prompt, options);
      const parsed = this.parseResponse(response.text);

      const trace: ReActTrace = {
        step: step + 1,
        thought: parsed.thought,
        action: parsed.action,
        actionInput: parsed.action_input,
        observation: null,
        timestamp: Date.now(),
      };

      // Check for final answer
      if (parsed.final_answer) {
        traces.push(trace);
        return {
          answer: parsed.final_answer,
          traces,
          toolCalls,
          iterations: step + 1,
        };
      }

      // Execute tool if specified
      if (parsed.action && parsed.action_input) {
        const observation = await this.executeTool(parsed.action, parsed.action_input);
        toolCalls++;
        trace.observation = observation;
      } else {
        // No action and no final answer - force conclusion
        traces.push(trace);
        return {
          answer: parsed.thought || 'I could not complete my reasoning.',
          traces,
          toolCalls,
          iterations: step + 1,
        };
      }

      traces.push(trace);
      scratchpad += `\nStep ${step + 1}:\n  Thought: ${parsed.thought}\n  Action: ${parsed.action}\n  Action Input: ${JSON.stringify(parsed.action_input)}\n  Observation: ${trace.observation}\n`;
    }

    // Max steps reached - ask for best answer
    const finalPrompt = `${this.systemPrompt}\n\nUser: ${userInput}\n\n${scratchpad}\n\nYou have reached max steps. Provide your best answer now in JSON format with "final_answer".`;
    const finalOptions: LLMOptions = { temperature: 0.3 };
    const finalResponse = await this.llm.complete(finalPrompt, finalOptions);
    const finalParsed = this.parseResponse(finalResponse.text);

    return {
      answer: finalParsed.final_answer || finalParsed.thought || 'Max reasoning steps reached.',
      traces,
      toolCalls,
      iterations: this.maxSteps,
    };
  }

  /** Execute a tool action. */
  private async executeTool(action: string, input: Record<string, unknown>): Promise<string> {
    try {
      switch (action) {
        case 'search_memory': {
          const query = input.query as string;
          const limit = (input.limit as number) || 5;
          const results = await this.memory.search(query, { limit });

          if (results.length === 0) {
            return 'No memories found matching this query.';
          }

          return results.map(r =>
            `[id: ${r.memory.id.substring(0, 8)}, score: ${r.score.toFixed(2)}, type: ${r.memory.type}] ${r.memory.content}`
          ).join('\n');
        }

        case 'add_memory': {
          const content = input.content as string;
          const type = (input.type as string) || 'fact';
          const importance = (input.importance as number) || 0.5;

          const mem = await this.memory.add(content, {
            type,
            importance,
            source: 'react-agent',
          });

          return `Memory stored. ID: ${mem.id.substring(0, 8)}, type: ${mem.type}, importance: ${mem.importance.toFixed(2)}`;
        }

        default:
          return `Unknown tool: ${action}`;
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      logger.warn({ action, error: message }, 'Tool execution failed');
      return `Tool error: ${message}`;
    }
  }

  /** Parse the LLM response into a structured ReActThought. */
  private parseResponse(text: string): ReActThought {
    try {
      const jsonMatch = text.match(/\{[\s\S]*\}/);
      if (jsonMatch) {
        const parsed = JSON.parse(jsonMatch[0]) as Record<string, unknown>;
        return {
          thought: (parsed.thought as string) || '',
          action: (parsed.action as string) || null,
          action_input: (parsed.action_input as Record<string, unknown>) || null,
          final_answer: (parsed.final_answer as string) || null,
        };
      }
    } catch {
      // Fallback to text parsing
    }

    // Fallback: parse free-text ReAct format
    const thoughtMatch = text.match(/Thought:\s*(.*?)(?:\n|$)/i);
    const actionMatch = text.match(/Action:\s*(.*?)(?:\n|$)/i);
    const inputMatch = text.match(/Action Input:\s*(.*?)(?:\n|$)/i);
    const answerMatch = text.match(/Final Answer:\s*([\s\S]*?)$/i);

    let actionInput: Record<string, unknown> | null = null;
    if (inputMatch) {
      try {
        actionInput = JSON.parse(inputMatch[1].trim()) as Record<string, unknown>;
      } catch {
        actionInput = { raw: inputMatch[1].trim() };
      }
    }

    return {
      thought: thoughtMatch?.[1] || text.substring(0, 200),
      action: actionMatch?.[1]?.trim() || null,
      action_input: actionInput,
      final_answer: answerMatch?.[1]?.trim() || null,
    };
  }
}
