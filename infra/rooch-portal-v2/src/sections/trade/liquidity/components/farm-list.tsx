import { useMemo, useState } from 'react';
import { useRoochClientQuery } from '@roochnetwork/rooch-sdk-kit';

import { Card, Table, TableBody, Typography } from '@mui/material';

import { useNetworkVariable } from 'src/hooks/use-networks';

import { Scrollbar } from 'src/components/scrollbar';
import WalletGuard from 'src/components/guard/WalletGuard';
import TableSkeleton from 'src/components/skeleton/table-skeleton';
import { TableNoData, TableHeadCustom } from 'src/components/table';

import { useAllLiquidity } from 'src/sections/trade/hooks/use-all-liquidity';
import { useOwnerLiquidity } from 'src/sections/trade/hooks/use-owner-liquidity';
import Box from '@mui/material/Box';
import FarmRowItem from './farm-row-item';
import AddStakeModal from './add-stake-modal';
import AddLiquidityModal from './add-liquidity-modal';

import type { FarmRowItemType } from './farm-row-item';

const headerLabel = [
  { id: 'lp', label: 'LP' },
  { id: 'release_per_second', label: 'Release Per Second' },
  { id: 'asset_total_weight', label: 'Total Staked LP' },
  { id: 'endtime', label: 'Endtime' },
  { id: 'action', label: '', align: 'right' },
];

export default function FarmList() {
  const dex = useNetworkVariable('dex');
  const [openStakeModal, setOpenStakeModal] = useState(false);
  const [openAddLiquidityModal, setOpenAddLiquidityModal] = useState(false);
  const [selectedRow, setSelectedRow] = useState<FarmRowItemType>();
  const { lpTokens } = useOwnerLiquidity();
  const { lpTokens: allLPTokens } = useAllLiquidity(200);

  const { data: farms, isPending } = useRoochClientQuery('queryObjectStates', {
    filter: {
      object_type: `${dex.address}::liquidity_incentive::FarmingAsset`,
    },
  });

  console.log(farms);

  const resolvedFarms = useMemo(() => {
    if (!farms) {
      return {
        valid: [],
        invalid: [],
      };
    }
    const now = Date.now() / 1000;
    const data = farms.data.map((item) => {
      const view = item.decoded_value!.value;
      const types = item.object_type
        .replace(`${dex.address}::liquidity_incentive::FarmingAsset<`, '')
        .trim()
        .split(',');
      const x = {
        type: types[0].trim(),
        name: types[0].split('::')[2].trim(),
      };
      const y = {
        type: types[1].trim(),
        name: types[1].split('::')[2].trim(),
      };
      return {
        id: item.id,
        alive: view.alive as boolean,
        endtime: view.end_time as number,
        assetTotalWeight: view.asset_total_weight as number,
        releasePerSecond: view.release_per_second as number,
        x,
        y,
        reward: types[2].replaceAll('>', '').trim(),
        liquidity: lpTokens.find((item) => item.x.type === x.type && item.y.type === y.type),
      };
    });

    const valid = data.filter((item) => item.endtime > now);
    const invalid = data.filter((item) => item.endtime < now);

    return {
      valid,
      invalid,
    };
  }, [farms, lpTokens, dex.address]);

  const handleOpenStakeModal = (row: FarmRowItemType) => {
    setSelectedRow(row);
    setOpenStakeModal(true);
  };

  const handleCloseStakeModal = () => {
    setOpenStakeModal(false);
    setSelectedRow(undefined);
  };

  const handleOpenAddLiquidityModal = (row: FarmRowItemType) => {
    setSelectedRow(row);
    setOpenAddLiquidityModal(true);
  };

  const handleCloseAddLiquidityModal = () => {
    setOpenAddLiquidityModal(false);
    setSelectedRow(undefined);
  };

  return (
    <WalletGuard>
      <Card className="mt-4">
        <Scrollbar sx={{ minHeight: 462 }}>
          <Table sx={{ minWidth: 720 }} size="medium">
            <TableHeadCustom headLabel={headerLabel} />

            <TableBody>
              {isPending ? (
                <TableSkeleton col={5} row={5} rowHeight="77px" />
              ) : (
                <>
                  {resolvedFarms.valid?.map((row) => (
                    <FarmRowItem
                      key={row.id}
                      row={row}
                      finished={false}
                      onOpenAddLiquidityModal={handleOpenAddLiquidityModal}
                      onOpenStakeModal={handleOpenStakeModal}
                      selectRow={selectedRow}
                    />
                  ))}
                  <TableNoData
                    title="No Farms Found"
                    notFound={resolvedFarms.valid?.length === 0}
                  />
                </>
              )}
            </TableBody>
          </Table>
        </Scrollbar>

        {selectedRow && (
          <AddStakeModal
            open={openStakeModal}
            onClose={handleCloseStakeModal}
            row={selectedRow}
            key={openStakeModal ? 'open' : 'closed'}
          />
        )}
        {selectedRow && (
          <AddLiquidityModal
            open={openAddLiquidityModal}
            onClose={handleCloseAddLiquidityModal}
            row={
              allLPTokens.find(
                (item) => item.x.type === selectedRow.x.type && item.y.type === selectedRow.y.type
              )!
            }
            key={openAddLiquidityModal ? 'open' : 'closed'}
          />
        )}
      </Card>
      <Box
        style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          height: '100px',
        }}
      >
        <Typography className="text-gray-600 !text-sm !font-semibold !mt-4">Finished</Typography>
      </Box>
      <Card className="mt-4">
        <Scrollbar sx={{ minHeight: 462 }}>
          <Table sx={{ minWidth: 720 }} size="medium">
            <TableHeadCustom headLabel={headerLabel} />

            <TableBody>
              {isPending ? (
                <TableSkeleton col={5} row={5} rowHeight="77px" />
              ) : (
                <>
                  {resolvedFarms.invalid?.map((row) => (
                    <FarmRowItem
                      key={row.id}
                      row={row}
                      finished
                      onOpenAddLiquidityModal={handleOpenAddLiquidityModal}
                      onOpenStakeModal={handleOpenStakeModal}
                      selectRow={selectedRow}
                    />
                  ))}
                  <TableNoData
                    title="No Farms Found"
                    notFound={resolvedFarms.invalid?.length === 0}
                  />
                </>
              )}
            </TableBody>
          </Table>
        </Scrollbar>

        {selectedRow && (
          <AddStakeModal
            open={openStakeModal}
            onClose={handleCloseStakeModal}
            row={selectedRow}
            key={openStakeModal ? 'open' : 'closed'}
          />
        )}
        {selectedRow && (
          <AddLiquidityModal
            open={openAddLiquidityModal}
            onClose={handleCloseAddLiquidityModal}
            row={
              allLPTokens.find(
                (item) => item.x.type === selectedRow.x.type && item.y.type === selectedRow.y.type
              )!
            }
            key={openAddLiquidityModal ? 'open' : 'closed'}
          />
        )}
      </Card>
    </WalletGuard>
  );
}
