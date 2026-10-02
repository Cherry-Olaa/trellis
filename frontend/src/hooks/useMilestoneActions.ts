import { useState } from 'react';
import type { Milestone, Agreement, EscrowStatus } from '../lib/soroban';

interface WalletLike {
  connected: boolean;
  publicKey: string | null;
  loading: boolean;
  error: string | null;
}

const STATUS_BADGE_COLORS: Record<EscrowStatus, string> = {
  Pending: 'bg-gray-600',
  Funded: 'bg-blue-600',
  WorkSubmitted: 'bg-yellow-600',
  Completed: 'bg-green-600',
  Disputed: 'bg-red-600',
  Refunded: 'bg-gray-600',
  Cancelled: 'bg-slate-500',
};

export function getStatusBadgeColor(status: EscrowStatus): string {
  return STATUS_BADGE_COLORS[status];
}

/**
 * Shared action/state logic behind both the desktop table row
 * (`MilestoneRow`) and the mobile card (`MilestoneCard`) for a single
 * milestone, so the two layouts never drift out of sync on what actions
 * are available or how they behave.
 */
export function useMilestoneActions(milestone: Milestone, agreement: Agreement, wallet: WalletLike) {
  const [actionLoading, setActionLoading] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [showProofInput, setShowProofInput] = useState(false);
  const [proofUri, setProofUri] = useState('');

  const isUserPayer = wallet.publicKey === agreement.payer;
  const isUserPayee = wallet.publicKey === agreement.payee;

  const handleLockFunds = async () => {
    if (!wallet.connected) {
      setActionError('Please connect your wallet');
      return;
    }

    setActionLoading(true);
    setActionError(null);

    try {
      // TODO: Implement contract call to lock_funds
      console.log('Lock funds action not yet implemented');
      setActionError('Lock funds action not yet implemented');
    } catch (error) {
      setActionError(error instanceof Error ? error.message : 'Failed to lock funds');
    } finally {
      setActionLoading(false);
    }
  };

  const handleSubmitWork = async () => {
    if (!wallet.connected) {
      setActionError('Please connect your wallet');
      return;
    }

    if (!proofUri.trim()) {
      setActionError('Please enter a proof URI');
      return;
    }

    setActionLoading(true);
    setActionError(null);

    try {
      // TODO: Implement contract call to submit_work
      console.log('Submit work action not yet implemented');
      setActionError('Submit work action not yet implemented');
      setShowProofInput(false);
      setProofUri('');
    } catch (error) {
      setActionError(error instanceof Error ? error.message : 'Failed to submit work');
    } finally {
      setActionLoading(false);
    }
  };

  const handleApproveRelease = async () => {
    if (!wallet.connected) {
      setActionError('Please connect your wallet');
      return;
    }

    setActionLoading(true);
    setActionError(null);

    try {
      // TODO: Implement contract call to approve_and_release
      console.log('Approve and release action not yet implemented');
      setActionError('Approve and release action not yet implemented');
    } catch (error) {
      setActionError(error instanceof Error ? error.message : 'Failed to approve and release');
    } finally {
      setActionLoading(false);
    }
  };

  const handleRaiseDispute = async () => {
    if (!wallet.connected) {
      setActionError('Please connect your wallet');
      return;
    }

    setActionLoading(true);
    setActionError(null);

    try {
      // TODO: Implement contract call to raise_dispute
      console.log('Raise dispute action not yet implemented');
      setActionError('Raise dispute action not yet implemented');
    } catch (error) {
      setActionError(error instanceof Error ? error.message : 'Failed to raise dispute');
    } finally {
      setActionLoading(false);
    }
  };

  const availableActions: Array<{ label: string; action: () => void; requiresWallet?: boolean }> = [];

  if (milestone.status === 'Pending' && isUserPayer) {
    availableActions.push({ label: 'Lock Funds', action: handleLockFunds, requiresWallet: true });
  }

  if (milestone.status === 'Funded' && isUserPayee) {
    availableActions.push({ label: 'Submit Work', action: handleSubmitWork, requiresWallet: true });
  }

  if (milestone.status === 'WorkSubmitted' && isUserPayer) {
    availableActions.push({ label: 'Approve & Release', action: handleApproveRelease, requiresWallet: true });
  }

  // raise_dispute rejects any caller other than the payer or payee.
  if ((milestone.status === 'Funded' || milestone.status === 'WorkSubmitted') && wallet.connected && (isUserPayer || isUserPayee)) {
    availableActions.push({ label: 'Raise Dispute', action: handleRaiseDispute, requiresWallet: true });
  }

  return {
    availableActions,
    actionLoading,
    actionError,
    showProofInput,
    setShowProofInput,
    proofUri,
    setProofUri,
    isUserPayee,
  };
}
