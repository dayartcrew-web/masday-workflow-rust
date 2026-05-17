/**
 * State Machine
 *
 * Manages valid workflow state transitions.
 * Ported from masday-workflow-reborn/packages/orchestrator/src/stateMachine.ts
 */

import type { Workflow, WorkflowState } from "@mcp-rebuild/core";
import { EventBus } from "@mcp-rebuild/core";
import { createLogger } from "@mcp-rebuild/core";

const logger = createLogger("StateMachine");

type StateTransition = {
  from: WorkflowState;
  to: WorkflowState;
  condition?: (workflow: Workflow) => boolean;
};

export class StateMachine {
  private transitions: Map<WorkflowState, WorkflowState[]> = new Map();
  private eventBus: EventBus;

  constructor(eventBus: EventBus) {
    this.eventBus = eventBus;
    this.setupTransitions();
  }

  private setupTransitions(): void {
    this.addTransition("INIT", "ANALYZE");
    this.addTransition("INIT", "DONE");
    this.addTransition("INIT", "FAILED");
    this.addTransition("ANALYZE", "PLAN");
    this.addTransition("ANALYZE", "FAILED");
    this.addTransition("PLAN", "EXECUTE");
    this.addTransition("PLAN", "FAILED");
    this.addTransition("EXECUTE", "VERIFY");
    this.addTransition("EXECUTE", "FIX");
    this.addTransition("VERIFY", "DONE");
    this.addTransition("VERIFY", "FIX");
    this.addTransition("FIX", "EXECUTE");
    this.addTransition("FIX", "DONE");
    this.addTransition("FIX", "FAILED");

    this.addTransition("EXECUTE", "PAUSED");
    this.addTransition("PAUSED", "EXECUTE");
    this.addTransition("PAUSED", "FAILED");
  }

  addTransition(from: WorkflowState, to: WorkflowState): void {
    if (!this.transitions.has(from)) {
      this.transitions.set(from, []);
    }
    this.transitions.get(from)!.push(to);
  }

  canTransition(from: WorkflowState, to: WorkflowState): boolean {
    const validTargets = this.transitions.get(from);
    return validTargets?.includes(to) ?? false;
  }

  transition(workflow: Workflow, to: WorkflowState): Workflow {
    if (!this.canTransition(workflow.state, to)) {
      throw new Error(
        `Invalid state transition from ${workflow.state} to ${to}`,
      );
    }

    const previousState = workflow.state;
    workflow.state = to;
    workflow.updatedAt = new Date();

    logger.info(
      { workflowId: workflow.id, traceId: workflow.traceId },
      `Workflow ${workflow.id} transitioned: ${previousState} -> ${to}`,
    );

    this.eventBus.emit("workflow.state.transition", {
      workflowId: workflow.id,
      traceId: workflow.traceId,
      from: previousState,
      to,
    });

    return workflow;
  }

  reset(workflow: Workflow): Workflow {
    const previousState = workflow.state;
    workflow.state = "INIT";
    workflow.updatedAt = new Date();
    logger.info(
      { workflowId: workflow.id, traceId: workflow.traceId },
      `Workflow ${workflow.id} reset to INIT`,
    );

    this.eventBus.emit("workflow.state.transition", {
      workflowId: workflow.id,
      traceId: workflow.traceId,
      from: previousState,
      to: "INIT",
    });

    return workflow;
  }
}
