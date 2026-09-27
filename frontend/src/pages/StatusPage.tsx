import { useCallback, useEffect, useState } from 'react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import type { Agreement, SorobanEvent } from '../lib/soroban';
import { getAgreement, sorobanServer } from '../lib/soroban';
import { useWallet } from '../lib/useWallet';
import { addToHistory } from '../lib/history';
import { isValidHexAgreementId } from '../lib/agreementId';
import { useAgreement } from '../hooks/useAgreement';
import { useAgreementEvents } from '../hooks/useAgreementEvents';
import { ExplorerLink } from '../components/ExplorerLink';
import MilestoneRow from '../components/MilestoneRow';
import MilestoneCard from '../components/MilestoneCard';
import LastUpdated from '../components/LastUpdated';
import { ExplorerLink } from '../components/ExplorerLink';
import { AgreementCardSkeleton } from '../components/skeletons';
import { ExplorerLink } from '../components/ExplorerLink';

export default function StatusPage() {
  const { id: urlId } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const wallet = useWallet();

  const queryId = searchParams.get('id');
  const initialId = urlId || queryId || '';

  const [agreementId, setAgreementId] = useState(initialId);
  // The id currently being fetched. `null` until a valid id is submitted, which
  // keeps the data hooks idle on first render.
  const [queriedId, setQueriedId] = useState<string | null>(
    initialId && isValidHexAgreementId(initialId.trim()) ? initialId.trim() : null,
  );
  const [validationError, setValidationError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  // Real on-chain reads — the previous local `queryAgreement` stub always threw,
  // so the page could never display an agreement (issue #251).
  const { agreement, isLoading, isError, error, refetch } = useAgreement(queriedId);
  const {
    events,
    isLoading: eventsLoading,
    error: eventsError,
  } = useAgreementEvents(queriedId);

  const runQuery = useCallback(
    (rawId: string) => {
      const id = rawId.trim();

      if (!id) {
        setValidationError('Please enter an agreement ID');
        return;
      }

      if (!isValidHexAgreementId(id)) {
        setValidationError(
          'Invalid agreement ID. Must be a 64-character hex string (0-9, a-f).',
        );
        return;
      }

      setValidationError(null);
      setQueriedId(id);
      navigate(`/agreement/${id}`);

      // Query agreement using contract read call
      const agreement = await getAgreement(id);
      setAgreement(agreement);

      // Query events
      const events = await queryEvents();
      setEvents(events);
      setLastUpdated(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to query agreement');
      setAgreement(null);
      setEvents([]);
    } finally {
      setLoading(false);
    }
  }, [navigate]);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    runQuery(agreementId);
  };

  useEffect(() => {
    if (initialId) {
      setAgreementId(initialId);
      runQuery(initialId);
    }
  }, [initialId, runQuery]);

  // Stamp the refresh time whenever a fresh agreement payload lands.
  useEffect(() => {
    if (agreement) setLastUpdated(new Date());
  }, [agreement]);

  // Record successful agreement views in local history
  useEffect(() => {
    if (!agreement) return;
    addToHistory({
      agreementId: agreement.agreement_id,
      lastViewed: new Date().toISOString(),
    });
  }, [agreement]);

  const displayError =
    validationError ?? (isError ? error ?? 'Failed to query agreement' : null);

  return (
    <div className="min-h-screen bg-gradient-to-b from-navy-900 via-navy-800 to-navy-900 dark:from-navy-900 dark:via-navy-800 dark:to-navy-900 light:from-white light:via-gray-50 light:to-white px-6 py-12">
      <div className="max-w-6xl mx-auto">
        <h1 className="text-3xl font-bold text-white dark:text-white light:text-gray-900 mb-8">Agreement Status</h1>

        {/* Search Form */}
        <form onSubmit={handleSearch} className="mb-8">
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Enter Agreement ID (hex format)"
              value={agreementId}
              onChange={(e) => setAgreementId(e.target.value)}
              className="min-w-0 flex-1 px-4 py-3 rounded-lg bg-navy-700 dark:bg-navy-700 light:bg-white border border-navy-600 dark:border-navy-600 light:border-gray-300 text-white dark:text-white light:text-gray-900 placeholder-gray-500 dark:placeholder-gray-500 light:placeholder-gray-400 focus:outline-none focus:border-cyan-400"
            />
            <button
              type="submit"
              disabled={isLoading}
              className="px-6 py-3 bg-cyan-400 text-navy-900 font-semibold rounded-lg hover:bg-cyan-300 transition-colors disabled:opacity-50"
            >
              {isLoading ? 'Loading...' : 'Search'}
            </button>
          </div>
          {displayError && <p className="mt-2 text-red-400 text-sm">{displayError}</p>}
        </form>

        {isLoading && !agreement && <AgreementCardSkeleton />}

        {!queriedId && !isLoading && !displayError && (
          <p className="text-gray-400 dark:text-gray-400 light:text-gray-600">
            Enter an agreement ID above to view its on-chain status.
          </p>
        )}

        {queriedId && !isLoading && !agreement && !displayError && (
          <p className="text-gray-400 dark:text-gray-400 light:text-gray-600">
            No agreement found for this ID.
          </p>
        )}

        {agreement && (
          <>
            {/* Timestamp of last on-chain data refresh — locale-independent (issue #101). */}
            {lastUpdated && (
              <div className="mb-4 flex justify-end">
                <LastUpdated lastUpdated={lastUpdated} />
              </div>
            )}

            {/* Agreement Details */}
            <div className="bg-navy-800 dark:bg-navy-800 light:bg-white border border-navy-700 dark:border-navy-700 light:border-gray-200 rounded-lg p-6 mb-8">
              <h2 className="text-xl font-semibold text-cyan-400 mb-6">Agreement Details</h2>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                  <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm mb-1">Payer</p>
                  <ExplorerLink type="account" value={agreement.payer} full />
                </div>
                <div>
                  <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm mb-1">Payee</p>
                  <ExplorerLink type="account" value={agreement.payee} full />
                </div>
                <div>
                  <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm mb-1">Token</p>
                  <ExplorerLink type="contract" value={agreement.token} full />
                </div>
                <div>
                  <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm mb-1">Dispute Resolver</p>
                  <ExplorerLink type="account" value={agreement.dispute_resolver} full />
                </div>
                <div>
                  <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm mb-1">Agreement ID</p>
                  <ExplorerLink type="agreement" value={agreement.agreement_id} full />
                </div>
                <div>
                  <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm mb-1">Milestones</p>
                  <p className="text-white dark:text-white light:text-gray-900 font-semibold">{agreement.milestones.length}</p>
                </div>
              </div>
            </div>

            {/* Milestones Table */}
            <div className="bg-navy-800 dark:bg-navy-800 light:bg-white border border-navy-700 dark:border-navy-700 light:border-gray-200 rounded-lg p-6 mb-8">
              <h2 className="text-xl font-semibold text-cyan-400 mb-6">Milestones</h2>

              {/* Desktop: full table. Mobile: single-column cards (#120) — a
                  scrollable table is hard to read and hard to tap on a phone. */}
              <div className="hidden md:block overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-navy-700 dark:border-navy-700 light:border-gray-200">
                      <th className="text-left py-3 px-4 text-gray-400 dark:text-gray-400 light:text-gray-600 font-semibold text-sm">ID</th>
                      <th className="text-left py-3 px-4 text-gray-400 dark:text-gray-400 light:text-gray-600 font-semibold text-sm">Amount</th>
                      <th className="text-left py-3 px-4 text-gray-400 dark:text-gray-400 light:text-gray-600 font-semibold text-sm">Status</th>
                      <th className="text-left py-3 px-4 text-gray-400 dark:text-gray-400 light:text-gray-600 font-semibold text-sm">Proof</th>
                      <th className="text-left py-3 px-4 text-gray-400 dark:text-gray-400 light:text-gray-600 font-semibold text-sm">Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {agreement.milestones.map((milestone, idx) => (
                      <MilestoneRow
                        key={idx}
                        milestone={milestone}
                        agreement={agreement}
                        wallet={wallet}
                        onUpdate={refetch}
                      />
                    ))}
                  </tbody>
                </table>
              </div>

              <div className="md:hidden space-y-3">
                {agreement.milestones.map((milestone) => (
                  <MilestoneCard
                    key={milestone.id}
                    milestone={milestone}
                    agreement={agreement}
                    wallet={wallet}
                    onUpdate={() => handleQuery(agreementId)}
                  />
                ))}
              </div>
            </div>

            {/* Event Feed */}
            <div className="bg-navy-800 dark:bg-navy-800 light:bg-white border border-navy-700 dark:border-navy-700 light:border-gray-200 rounded-lg p-6">
              <h2 className="text-xl font-semibold text-cyan-400 mb-6">Event History</h2>

              {eventsError ? (
                <p className="text-red-400 text-sm">{eventsError}</p>
              ) : eventsLoading && events.length === 0 ? (
                <p className="text-gray-400 dark:text-gray-400 light:text-gray-600">Loading events…</p>
              ) : events.length === 0 ? (
                <p className="text-gray-400 dark:text-gray-400 light:text-gray-600">No events yet</p>
              ) : (
                <div className="space-y-4">
                  {events.map((event, idx) => (
                    <div key={idx} className="border-l-4 border-cyan-400 pl-4 py-2">
                      <p className="text-cyan-400 font-semibold text-sm">{event.type}</p>
                      <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm">Ledger: {event.ledger}</p>
                      <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm">
                        {new Date(event.timestamp).toLocaleString()}
                      </p>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

async function queryEvents(): Promise<SorobanEvent[]> {
  // Query events from Soroban RPC
  // This is a simplified implementation
  try {
    const events = await sorobanServer.getEvents({
      startLedger: 0,
      limit: 100,
      filters: [
        {
          type: 'contract',
          contractIds: [CONTRACT_ID],
        },
      ],
    });

    const rawEvents: unknown[] = events.events || [];
    return rawEvents
      .filter(isValidEventResponse)
      .map((event: RawEventResponse) => ({
        type: 'event',
        ledger: event.ledger || 0,
        txHash: event.txHash || '',
        timestamp: Date.now(),
        data: {},
      }));
  } catch (error) {
    console.error('Failed to fetch events:', error);
    return [];
  }
}
