/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of SSD1-00000000.aml
 *
 * Original Table Header:
 *     Signature        "SSDT"
 *     Length           0x00003F53 (16211)
 *     Revision         0x02
 *     Checksum         0x1C
 *     OEM ID           "DptfTb"
 *     OEM Table ID     "DptfTabl"
 *     OEM Revision     0x00001000 (4096)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20160422 (538313762)
 */
DefinitionBlock ("", "SSDT", 2, "DptfTb", "DptfTabl", 0x00001000)
{
    External (_SB_.AAC0, FieldUnitObj)
    External (_SB_.ACRT, FieldUnitObj)
    External (_SB_.APSV, FieldUnitObj)
    External (_SB_.CBMI, FieldUnitObj)
    External (_SB_.CFGD, FieldUnitObj)
    External (_SB_.CLVL, FieldUnitObj)
    External (_SB_.CPPC, FieldUnitObj)
    External (_SB_.CTC0, FieldUnitObj)
    External (_SB_.CTC1, FieldUnitObj)
    External (_SB_.CTC2, FieldUnitObj)
    External (_SB_.OSCP, IntObj)
    External (_SB_.PAGD, DeviceObj)
    External (_SB_.PAGD._PUR, PkgObj)
    External (_SB_.PAGD._STA, MethodObj)    // 0 Arguments
    External (_SB_.PC00, DeviceObj)
    External (_SB_.PC00.LPCB.H_EC, DeviceObj)
    External (_SB_.PC00.LPCB.H_EC.ACUR, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.AP01, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.AP02, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.AP10, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.ARTG, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.AVOL, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.B1FC, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.B1RC, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.BICC, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.BMAX, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.CFAN, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.CFSP, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.CHGR, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.CHRG, DeviceObj)
    External (_SB_.PC00.LPCB.H_EC.CMDR, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.CMPP, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.CPUP, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.CTYP, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.ECAV, IntObj)
    External (_SB_.PC00.LPCB.H_EC.ECF2, OpRegionObj)
    External (_SB_.PC00.LPCB.H_EC.FCHG, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.HYST, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PBSS, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PECH, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PENV, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PINV, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PLMX, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PMAX, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PPSH, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PPSL, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PPWR, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PROP, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PSOC, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PSTP, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.PWRT, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.RBHF, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TAMB, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TDTS, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TPRV, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TSHT, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TSI_, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TSLT, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TSR1, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TSR2, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TSR3, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.TSSR, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.UVTH, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.VBNL, FieldUnitObj)
    External (_SB_.PC00.LPCB.H_EC.VMIN, FieldUnitObj)
    External (_SB_.PC00.MHBR, FieldUnitObj)
    External (_SB_.PC00.TCPU, DeviceObj)
    External (_SB_.PL10, FieldUnitObj)
    External (_SB_.PL11, FieldUnitObj)
    External (_SB_.PL12, FieldUnitObj)
    External (_SB_.PL20, FieldUnitObj)
    External (_SB_.PL21, FieldUnitObj)
    External (_SB_.PL22, FieldUnitObj)
    External (_SB_.PLW0, FieldUnitObj)
    External (_SB_.PLW1, FieldUnitObj)
    External (_SB_.PLW2, FieldUnitObj)
    External (_SB_.PR00, ProcessorObj)
    External (_SB_.PR00._PSS, MethodObj)    // 0 Arguments
    External (_SB_.PR00._TPC, IntObj)
    External (_SB_.PR00._TSD, MethodObj)    // 0 Arguments
    External (_SB_.PR00._TSS, MethodObj)    // 0 Arguments
    External (_SB_.PR00.LPSS, PkgObj)
    External (_SB_.PR00.TPSS, PkgObj)
    External (_SB_.PR00.TSMC, PkgObj)
    External (_SB_.PR00.TSMF, PkgObj)
    External (_SB_.PR01, ProcessorObj)
    External (_SB_.PR02, ProcessorObj)
    External (_SB_.PR03, ProcessorObj)
    External (_SB_.PR04, ProcessorObj)
    External (_SB_.PR05, ProcessorObj)
    External (_SB_.PR06, ProcessorObj)
    External (_SB_.PR07, ProcessorObj)
    External (_SB_.PR08, ProcessorObj)
    External (_SB_.PR09, ProcessorObj)
    External (_SB_.PR10, ProcessorObj)
    External (_SB_.PR11, ProcessorObj)
    External (_SB_.PR12, ProcessorObj)
    External (_SB_.PR13, ProcessorObj)
    External (_SB_.PR14, ProcessorObj)
    External (_SB_.PR15, ProcessorObj)
    External (_SB_.SLPB, DeviceObj)
    External (_SB_.TAR0, FieldUnitObj)
    External (_SB_.TAR1, FieldUnitObj)
    External (_SB_.TAR2, FieldUnitObj)
    External (_TZ_.ETMD, IntObj)
    External (_TZ_.TZ00, ThermalZoneObj)
    External (ACTT, IntObj)
    External (ATPC, IntObj)
    External (BATR, IntObj)
    External (CHGE, IntObj)
    External (CRTT, IntObj)
    External (DCFE, IntObj)
    External (DPTF, IntObj)
    External (ECON, IntObj)
    External (FND1, IntObj)
    External (HIDW, MethodObj)    // 4 Arguments
    External (HIWC, MethodObj)    // 1 Arguments
    External (IN34, IntObj)
    External (IPCS, MethodObj)    // 7 Arguments
    External (ODV0, IntObj)
    External (ODV1, IntObj)
    External (ODV2, IntObj)
    External (ODV3, IntObj)
    External (ODV4, IntObj)
    External (ODV5, IntObj)
    External (PCHE, FieldUnitObj)
    External (PF00, IntObj)
    External (PNHM, IntObj)
    External (PPPR, IntObj)
    External (PPSZ, IntObj)
    External (PSVT, IntObj)
    External (PTPC, IntObj)
    External (PWRE, IntObj)
    External (PWRS, IntObj)
    External (S1DE, IntObj)
    External (S2DE, IntObj)
    External (S2PT, IntObj)
    External (S3DE, IntObj)
    External (S4DE, IntObj)
    External (S5DE, IntObj)
    External (SADE, IntObj)
    External (SSP2, IntObj)
    External (SSP3, IntObj)
    External (SSP4, IntObj)
    External (SSP5, IntObj)
    External (TCNT, IntObj)
    External (TSOD, IntObj)

    Scope (\_SB)
    {
        Device (IETM)
        {
            Name (_HID, "INTC1040")  // _HID: Hardware ID
            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                If (CondRefOf (HIWC))
                {
                    If (HIWC (Arg0))
                    {
                        If (CondRefOf (HIDW))
                        {
                            Return (HIDW (Arg0, Arg1, Arg2, Arg3))
                        }
                    }
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If (((\DPTF == One) && (\IN34 == One)))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Name (PTRP, Zero)
            Name (PSEM, Zero)
            Name (ATRP, Zero)
            Name (ASEM, Zero)
            Name (YTRP, Zero)
            Name (YSEM, Zero)
            Method (_OSC, 4, Serialized)  // _OSC: Operating System Capabilities
            {
                CreateDWordField (Arg3, Zero, STS1)
                CreateDWordField (Arg3, 0x04, CAP1)
                If ((Arg1 != One))
                {
                    STS1 &= 0xFFFFFF00
                    STS1 |= 0x0A
                    Return (Arg3)
                }

                If ((Arg2 != 0x02))
                {
                    STS1 &= 0xFFFFFF00
                    STS1 |= 0x02
                    Return (Arg3)
                }

                If (CondRefOf (\_SB.APSV))
                {
                    If ((PSEM == Zero))
                    {
                        PSEM = One
                        PTRP = \_SB.APSV /* External reference */
                    }
                }

                If (CondRefOf (\_SB.AAC0))
                {
                    If ((ASEM == Zero))
                    {
                        ASEM = One
                        ATRP = \_SB.AAC0 /* External reference */
                    }
                }

                If (CondRefOf (\_SB.ACRT))
                {
                    If ((YSEM == Zero))
                    {
                        YSEM = One
                        YTRP = \_SB.ACRT /* External reference */
                    }
                }

                If ((Arg0 == ToUUID ("b23ba85d-c8b7-3542-88de-8de2ffcfd698") /* Unknown UUID */))
                {
                    If (~(STS1 & One))
                    {
                        If ((CAP1 & One))
                        {
                            If ((CAP1 & 0x02))
                            {
                                \_SB.AAC0 = 0x6E
                                \_TZ.ETMD = Zero
                            }
                            Else
                            {
                                \_SB.AAC0 = ATRP /* \_SB_.IETM.ATRP */
                                \_TZ.ETMD = One
                            }

                            If ((CAP1 & 0x04))
                            {
                                \_SB.APSV = 0x6E
                            }
                            Else
                            {
                                \_SB.APSV = PTRP /* \_SB_.IETM.PTRP */
                            }

                            If ((CAP1 & 0x08))
                            {
                                \_SB.ACRT = 0xD2
                            }
                            Else
                            {
                                \_SB.ACRT = YTRP /* \_SB_.IETM.YTRP */
                            }
                        }
                        Else
                        {
                            \_SB.ACRT = YTRP /* \_SB_.IETM.YTRP */
                            \_SB.APSV = PTRP /* \_SB_.IETM.PTRP */
                            \_SB.AAC0 = ATRP /* \_SB_.IETM.ATRP */
                            \_TZ.ETMD = One
                        }

                        If (CondRefOf (\_TZ.TZ00))
                        {
                            Notify (\_TZ.TZ00, 0x81) // Information Change
                        }
                    }

                    Return (Arg3)
                }

                Return (Arg3)
            }

            Method (DCFG, 0, NotSerialized)
            {
                Return (\DCFE) /* External reference */
            }

            Name (ODVX, Package (0x06)
            {
                Zero, 
                Zero, 
                Zero, 
                Zero, 
                Zero, 
                Zero
            })
            Method (ODVP, 0, Serialized)
            {
                ODVX [Zero] = \ODV0 /* External reference */
                ODVX [One] = \ODV1 /* External reference */
                ODVX [0x02] = \ODV2 /* External reference */
                ODVX [0x03] = \ODV3 /* External reference */
                ODVX [0x04] = \ODV4 /* External reference */
                ODVX [0x05] = \ODV5 /* External reference */
                Return (ODVX) /* \_SB_.IETM.ODVX */
            }
        }
    }

    Scope (\_SB.PC00.LPCB.H_EC)
    {
        Mutex (PATM, 0x00)
    }

    Scope (\_SB)
    {
        Device (BAT1)
        {
            Name (_HID, "INTC1050")  // _HID: Hardware ID
            Name (_UID, "1")  // _UID: Unique ID
            Name (_STR, Unicode ("Battery 1 Participant"))  // _STR: Description String
            Name (PTYP, 0x0C)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((\BATR == One))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Method (PMAX, 0, Serialized)
            {
                Return (Zero)
            }

            Method (CTYP, 0, NotSerialized)
            {
                Return (0x03)
            }

            Method (PBSS, 0, NotSerialized)
            {
                Return (0x64)
            }

            Method (DPSP, 0, Serialized)
            {
                Return (\PPPR) /* External reference */
            }

            Method (RBHF, 0, NotSerialized)
            {
                Return (0xFFFFFFFF)
            }

            Method (VBNL, 0, NotSerialized)
            {
                Return (0xFFFFFFFF)
            }

            Method (CMPP, 0, NotSerialized)
            {
                Return (0xFFFFFFFF)
            }
        }
    }

    Scope (\_SB.PC00.LPCB.H_EC)
    {
        Device (SEN1)
        {
            Name (_HID, "INTC1043")  // _HID: Hardware ID
            Name (_UID, "SEN1")  // _UID: Unique ID
            Name (_STR, Unicode ("CPU temperature"))  // _STR: Description String
            Name (PTYP, 0x03)
            Name (CTYP, Zero)
            Name (PFLG, Zero)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((\S1DE == One))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Method (_TMP, 0, Serialized)  // _TMP: Temperature
            {
                If (\_SB.PC00.LPCB.H_EC.ECAV)
                {
                    Return (\_SB.IETM.CTOK (\_SB.PC00.LPCB.H_EC.TSR2))
                }
                Else
                {
                    Return (0x0BB8)
                }
            }

            Name (PATC, 0x02)
            Method (PAT0, 1, Serialized)
            {
            }

            Method (PAT1, 1, Serialized)
            {
            }

            Name (GTSH, 0x14)
            Name (LSTM, Zero)
            Method (_DTI, 1, NotSerialized)  // _DTI: Device Temperature Indication
            {
                LSTM = Arg0
                Notify (\_SB.PC00.LPCB.H_EC.SEN1, 0x91) // Device-Specific
            }

            Method (_NTT, 0, NotSerialized)  // _NTT: Notification Temperature Threshold
            {
                Return (0x0ADE)
            }

            Name (S1AC, 0x3C)
            Name (S1PV, 0x41)
            Name (S1CC, 0x50)
            Name (S1C3, 0x46)
            Name (S1HP, 0x4B)
            Name (SSP1, 0x0A)
            Method (_TSP, 0, Serialized)  // _TSP: Thermal Sampling Period
            {
                Return (SSP1) /* \_SB_.PC00.LPCB.H_EC.SEN1.SSP1 */
            }

            Method (_AC0, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Local1 = \_SB.IETM.CTOK (S1AC)
                If ((LSTM >= Local1))
                {
                    Return ((Local1 - 0x14))
                }
                Else
                {
                    Return (Local1)
                }
            }

            Method (_PSV, 0, Serialized)  // _PSV: Passive Temperature
            {
                Return (\_SB.IETM.CTOK (S1PV))
            }

            Method (_CRT, 0, Serialized)  // _CRT: Critical Temperature
            {
                Return (\_SB.IETM.CTOK (S1CC))
            }

            Method (_CR3, 0, Serialized)  // _CR3: Warm/Standby Temperature
            {
                Return (\_SB.IETM.CTOK (S1C3))
            }

            Method (_HOT, 0, Serialized)  // _HOT: Hot Temperature
            {
                Return (\_SB.IETM.CTOK (S1HP))
            }
        }
    }

    Scope (\_SB.PC00.LPCB.H_EC)
    {
        Device (SEN2)
        {
            Name (_HID, "INTC1043")  // _HID: Hardware ID
            Name (_UID, "SEN2")  // _UID: Unique ID
            Name (_STR, Unicode ("SSD temperature"))  // _STR: Description String
            Name (PTYP, 0x03)
            Name (CTYP, Zero)
            Name (PFLG, Zero)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((\S2DE == One))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Method (_TMP, 0, Serialized)  // _TMP: Temperature
            {
                If (\_SB.PC00.LPCB.H_EC.ECAV)
                {
                    Return (\_SB.IETM.CTOK (\_SB.PC00.LPCB.H_EC.TSR3))
                }
                Else
                {
                    Return (0x0BB8)
                }
            }

            Name (PATC, 0x02)
            Method (PAT0, 1, Serialized)
            {
            }

            Method (PAT1, 1, Serialized)
            {
            }

            Name (GTSH, 0x14)
            Name (LSTM, Zero)
            Method (_DTI, 1, NotSerialized)  // _DTI: Device Temperature Indication
            {
                LSTM = Arg0
                Notify (\_SB.PC00.LPCB.H_EC.SEN2, 0x91) // Device-Specific
            }

            Method (_NTT, 0, NotSerialized)  // _NTT: Notification Temperature Threshold
            {
                Return (0x0ADE)
            }

            Name (S2AC, 0x3C)
            Name (S2A1, 0x32)
            Name (S2A2, 0x28)
            Name (S2PV, 0x41)
            Name (S2CC, 0x50)
            Name (S2C3, 0x46)
            Name (S2HP, 0x4B)
            Name (SSP2, Zero)
            Method (_TSP, 0, Serialized)  // _TSP: Thermal Sampling Period
            {
                Return (SSP2) /* \_SB_.PC00.LPCB.H_EC.SEN2.SSP2 */
            }

            Method (_AC0, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Local1 = \_SB.IETM.CTOK (S2AC)
                If ((LSTM >= Local1))
                {
                    Return ((Local1 - 0x14))
                }
                Else
                {
                    Return (Local1)
                }
            }

            Method (_AC1, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S2A1))
            }

            Method (_AC2, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S2A2))
            }

            Method (_PSV, 0, Serialized)  // _PSV: Passive Temperature
            {
                Return (\_SB.IETM.CTOK (S2PV))
            }

            Method (_CRT, 0, Serialized)  // _CRT: Critical Temperature
            {
                Return (\_SB.IETM.CTOK (S2CC))
            }

            Method (_CR3, 0, Serialized)  // _CR3: Warm/Standby Temperature
            {
                Return (\_SB.IETM.CTOK (S2C3))
            }

            Method (_HOT, 0, Serialized)  // _HOT: Hot Temperature
            {
                Return (\_SB.IETM.CTOK (S2HP))
            }
        }
    }

    Scope (\_SB.PC00.LPCB.H_EC)
    {
        Device (SEN3)
        {
            Name (_HID, "INTC1043")  // _HID: Hardware ID
            Name (_UID, "SEN3")  // _UID: Unique ID
            Name (_STR, Unicode ("Charger temperature"))  // _STR: Description String
            Name (PTYP, 0x03)
            Name (CTYP, Zero)
            Name (PFLG, Zero)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((\S3DE == One))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Method (_TMP, 0, Serialized)  // _TMP: Temperature
            {
                If (\_SB.PC00.LPCB.H_EC.ECAV)
                {
                    Return (\_SB.IETM.CTOK (\_SB.PC00.LPCB.H_EC.TPRV))
                }
                Else
                {
                    Return (0x0BB8)
                }
            }

            Name (PATC, 0x02)
            Method (PAT0, 1, Serialized)
            {
            }

            Method (PAT1, 1, Serialized)
            {
            }

            Name (GTSH, 0x14)
            Name (LSTM, Zero)
            Method (_DTI, 1, NotSerialized)  // _DTI: Device Temperature Indication
            {
                LSTM = Arg0
                Notify (\_SB.PC00.LPCB.H_EC.SEN3, 0x91) // Device-Specific
            }

            Method (_NTT, 0, NotSerialized)  // _NTT: Notification Temperature Threshold
            {
                Return (0x0ADE)
            }

            Name (S3AC, 0x3C)
            Name (S3A1, 0x32)
            Name (S3A2, 0x28)
            Name (S3PV, 0x41)
            Name (S3CC, 0x50)
            Name (S3C3, 0x46)
            Name (S3HP, 0x4B)
            Name (SSP3, Zero)
            Method (_TSP, 0, Serialized)  // _TSP: Thermal Sampling Period
            {
                Return (SSP3) /* \_SB_.PC00.LPCB.H_EC.SEN3.SSP3 */
            }

            Method (_AC3, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Local1 = \_SB.IETM.CTOK (S3AC)
                If ((LSTM >= Local1))
                {
                    Return ((Local1 - 0x14))
                }
                Else
                {
                    Return (Local1)
                }
            }

            Method (_AC4, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S3A1))
            }

            Method (_AC5, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S3A2))
            }

            Method (_PSV, 0, Serialized)  // _PSV: Passive Temperature
            {
                Return (\_SB.IETM.CTOK (S3PV))
            }

            Method (_CRT, 0, Serialized)  // _CRT: Critical Temperature
            {
                Return (\_SB.IETM.CTOK (S3CC))
            }

            Method (_CR3, 0, Serialized)  // _CR3: Warm/Standby Temperature
            {
                Return (\_SB.IETM.CTOK (S3C3))
            }

            Method (_HOT, 0, Serialized)  // _HOT: Hot Temperature
            {
                Return (\_SB.IETM.CTOK (S3HP))
            }
        }
    }

    Scope (\_SB.PC00.LPCB.H_EC)
    {
        Device (SEN4)
        {
            Name (_HID, "INTC1043")  // _HID: Hardware ID
            Name (_UID, "SEN4")  // _UID: Unique ID
            Name (_STR, Unicode ("ALS temperature"))  // _STR: Description String
            Name (PTYP, 0x03)
            Name (CTYP, Zero)
            Name (PFLG, Zero)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((\S4DE == One))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Method (_TMP, 0, Serialized)  // _TMP: Temperature
            {
                If (\_SB.PC00.LPCB.H_EC.ECAV)
                {
                    Return (\_SB.IETM.CTOK (\_SB.PC00.LPCB.H_EC.TAMB))
                }
                Else
                {
                    Return (0x0BB8)
                }
            }

            Name (PATC, 0x02)
            Method (PAT0, 1, Serialized)
            {
            }

            Method (PAT1, 1, Serialized)
            {
            }

            Name (GTSH, 0x14)
            Name (LSTM, Zero)
            Method (_DTI, 1, NotSerialized)  // _DTI: Device Temperature Indication
            {
                LSTM = Arg0
                Notify (\_SB.PC00.LPCB.H_EC.SEN4, 0x91) // Device-Specific
            }

            Method (_NTT, 0, NotSerialized)  // _NTT: Notification Temperature Threshold
            {
                Return (0x0ADE)
            }

            Name (S4AC, 0x3C)
            Name (S4A1, 0x32)
            Name (S4A2, 0x28)
            Name (S4PV, 0x41)
            Name (S4CC, 0x50)
            Name (S4C3, 0x46)
            Name (S4HP, 0x4B)
            Name (SSP4, Zero)
            Method (_TSP, 0, Serialized)  // _TSP: Thermal Sampling Period
            {
                Return (SSP4) /* \_SB_.PC00.LPCB.H_EC.SEN4.SSP4 */
            }

            Method (_AC0, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Local1 = \_SB.IETM.CTOK (S4AC)
                If ((LSTM >= Local1))
                {
                    Return ((Local1 - 0x14))
                }
                Else
                {
                    Return (Local1)
                }
            }

            Method (_AC1, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S4A1))
            }

            Method (_AC2, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S4A2))
            }

            Method (_PSV, 0, Serialized)  // _PSV: Passive Temperature
            {
                Return (\_SB.IETM.CTOK (S4PV))
            }

            Method (_CRT, 0, Serialized)  // _CRT: Critical Temperature
            {
                Return (\_SB.IETM.CTOK (S4CC))
            }

            Method (_CR3, 0, Serialized)  // _CR3: Warm/Standby Temperature
            {
                Return (\_SB.IETM.CTOK (S4C3))
            }

            Method (_HOT, 0, Serialized)  // _HOT: Hot Temperature
            {
                Return (\_SB.IETM.CTOK (S4HP))
            }
        }
    }

    Scope (\_SB.PC00.LPCB.H_EC)
    {
        Device (SEN5)
        {
            Name (_HID, "INTC1043")  // _HID: Hardware ID
            Name (_UID, "SEN5")  // _UID: Unique ID
            Name (_STR, Unicode ("ALS temperature"))  // _STR: Description String
            Name (PTYP, 0x03)
            Name (CTYP, Zero)
            Name (PFLG, Zero)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((\S5DE == One))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Method (_TMP, 0, Serialized)  // _TMP: Temperature
            {
                If (\_SB.PC00.LPCB.H_EC.ECAV)
                {
                    Return (\_SB.IETM.CTOK (\_SB.PC00.LPCB.H_EC.TAMB))
                }
                Else
                {
                    Return (0x0BB8)
                }
            }

            Name (PATC, 0x02)
            Method (PAT0, 1, Serialized)
            {
            }

            Method (PAT1, 1, Serialized)
            {
            }

            Name (GTSH, 0x14)
            Name (LSTM, Zero)
            Method (_DTI, 1, NotSerialized)  // _DTI: Device Temperature Indication
            {
                LSTM = Arg0
                Notify (\_SB.PC00.LPCB.H_EC.SEN5, 0x91) // Device-Specific
            }

            Method (_NTT, 0, NotSerialized)  // _NTT: Notification Temperature Threshold
            {
                Return (0x0ADE)
            }

            Name (S5AC, 0x3C)
            Name (S5A1, 0x32)
            Name (S5A2, 0x28)
            Name (S5PV, 0x41)
            Name (S5CC, 0x50)
            Name (S5C3, 0x46)
            Name (S5HP, 0x4B)
            Name (SSP5, Zero)
            Method (_TSP, 0, Serialized)  // _TSP: Thermal Sampling Period
            {
                Return (SSP5) /* \_SB_.PC00.LPCB.H_EC.SEN5.SSP5 */
            }

            Method (_AC0, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Local1 = \_SB.IETM.CTOK (S5AC)
                If ((LSTM >= Local1))
                {
                    Return ((Local1 - 0x14))
                }
                Else
                {
                    Return (Local1)
                }
            }

            Method (_AC1, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S5A1))
            }

            Method (_AC2, 0, Serialized)  // _ACx: Active Cooling, x=0-9
            {
                Return (\_SB.IETM.CTOK (S5A2))
            }

            Method (_PSV, 0, Serialized)  // _PSV: Passive Temperature
            {
                Return (\_SB.IETM.CTOK (S5PV))
            }

            Method (_CRT, 0, Serialized)  // _CRT: Critical Temperature
            {
                Return (\_SB.IETM.CTOK (S5CC))
            }

            Method (_CR3, 0, Serialized)  // _CR3: Warm/Standby Temperature
            {
                Return (\_SB.IETM.CTOK (S5C3))
            }

            Method (_HOT, 0, Serialized)  // _HOT: Hot Temperature
            {
                Return (\_SB.IETM.CTOK (S5HP))
            }
        }
    }

    Scope (\_SB.IETM)
    {
        Method (KTOC, 1, Serialized)
        {
            If ((Arg0 > 0x0AAC))
            {
                Return (((Arg0 - 0x0AAC) / 0x0A))
            }
            Else
            {
                Return (Zero)
            }
        }

        Method (CTOK, 1, Serialized)
        {
            Return (((Arg0 * 0x0A) + 0x0AAC))
        }

        Method (C10K, 1, Serialized)
        {
            Name (TMP1, Buffer (0x10)
            {
                 0x00                                             // .
            })
            CreateByteField (TMP1, Zero, TMPL)
            CreateByteField (TMP1, One, TMPH)
            Local0 = (Arg0 + 0x0AAC)
            TMPL = (Local0 & 0xFF)
            TMPH = ((Local0 & 0xFF00) >> 0x08)
            ToInteger (TMP1, Local1)
            Return (Local1)
        }

        Method (K10C, 1, Serialized)
        {
            If ((Arg0 > 0x0AAC))
            {
                Return ((Arg0 - 0x0AAC))
            }
            Else
            {
                Return (Zero)
            }
        }
    }

    Scope (\_SB.PC00.TCPU)
    {
        Name (PFLG, Zero)
        Method (_STA, 0, NotSerialized)  // _STA: Status
        {
            If ((\SADE == One))
            {
                Return (0x0F)
            }
            Else
            {
                Return (Zero)
            }
        }

        OperationRegion (CPWR, SystemMemory, ((MHBR << 0x0F) + 0x5000), 0x1000)
        Field (CPWR, ByteAcc, NoLock, Preserve)
        {
            Offset (0x930), 
            PTDP,   15, 
            Offset (0x932), 
            PMIN,   15, 
            Offset (0x934), 
            PMAX,   15, 
            Offset (0x936), 
            TMAX,   7, 
            Offset (0x938), 
            PWRU,   4, 
            Offset (0x939), 
            EGYU,   5, 
            Offset (0x93A), 
            TIMU,   4, 
            Offset (0x958), 
            Offset (0x95C), 
            LPMS,   1, 
            CTNL,   2, 
            Offset (0x978), 
            PCTP,   8, 
            Offset (0x998), 
            RP0C,   8, 
            RP1C,   8, 
            RPNC,   8, 
            Offset (0xF3C), 
            TRAT,   8, 
            Offset (0xF40), 
            PTD1,   15, 
            Offset (0xF42), 
            TRA1,   8, 
            Offset (0xF44), 
            PMX1,   15, 
            Offset (0xF46), 
            PMN1,   15, 
            Offset (0xF48), 
            PTD2,   15, 
            Offset (0xF4A), 
            TRA2,   8, 
            Offset (0xF4C), 
            PMX2,   15, 
            Offset (0xF4E), 
            PMN2,   15, 
            Offset (0xF50), 
            CTCL,   2, 
                ,   29, 
            CLCK,   1, 
            MNTR,   8
        }

        Name (XPCC, Zero)
        Method (PPCC, 0, Serialized)
        {
            If (((XPCC == Zero) && CondRefOf (\_SB.CBMI)))
            {
                Switch (ToInteger (\_SB.CBMI))
                {
                    Case (Zero)
                    {
                        If (((\_SB.CLVL >= One) && (\_SB.CLVL <= 0x03)))
                        {
                            CPL0 ()
                            XPCC = One
                        }
                    }
                    Case (One)
                    {
                        If (((\_SB.CLVL == 0x02) || (\_SB.CLVL == 0x03)))
                        {
                            CPL1 ()
                            XPCC = One
                        }
                    }
                    Case (0x02)
                    {
                        If ((\_SB.CLVL == 0x03))
                        {
                            CPL2 ()
                            XPCC = One
                        }
                    }

                }
            }

            Return (NPCC) /* \_SB_.PC00.TCPU.NPCC */
        }

        Name (NPCC, Package (0x03)
        {
            0x02, 
            Package (0x06)
            {
                Zero, 
                0x88B8, 
                0xAFC8, 
                0x6D60, 
                0x7D00, 
                0x03E8
            }, 

            Package (0x06)
            {
                One, 
                0xDBBA, 
                0xDBBA, 
                Zero, 
                Zero, 
                0x03E8
            }
        })
        Method (CPNU, 2, Serialized)
        {
            Name (CNVT, Zero)
            Name (PPUU, Zero)
            Name (RMDR, Zero)
            If ((PWRU == Zero))
            {
                PPUU = One
            }
            Else
            {
                PPUU = (PWRU-- << 0x02)
            }

            Divide (Arg0, PPUU, RMDR, CNVT) /* \_SB_.PC00.TCPU.CPNU.CNVT */
            If ((Arg1 == Zero))
            {
                Return (CNVT) /* \_SB_.PC00.TCPU.CPNU.CNVT */
            }
            Else
            {
                CNVT *= 0x03E8
                RMDR *= 0x03E8
                RMDR /= PPUU
                CNVT += RMDR /* \_SB_.PC00.TCPU.CPNU.RMDR */
                Return (CNVT) /* \_SB_.PC00.TCPU.CPNU.CNVT */
            }
        }

        Method (CPL0, 0, NotSerialized)
        {
            \_SB.PC00.TCPU.NPCC [Zero] = 0x02
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [Zero] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [One] = 0x7D
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x02] = CPNU (\_SB.PL10, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x03] = (\_SB.PLW0 * 0x03E8)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x04] = ((\_SB.PLW0 * 0x03E8
                ) + 0x0FA0)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x05] = PPSZ /* External reference */
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [Zero] = One
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [One] = CPNU (\_SB.PL20, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x02] = CPNU (\_SB.PL20, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x03] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x04] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x05] = PPSZ /* External reference */
        }

        Method (CPL1, 0, NotSerialized)
        {
            \_SB.PC00.TCPU.NPCC [Zero] = 0x02
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [Zero] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [One] = 0x7D
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x02] = CPNU (\_SB.PL11, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x03] = (\_SB.PLW1 * 0x03E8)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x04] = ((\_SB.PLW1 * 0x03E8
                ) + 0x0FA0)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x05] = PPSZ /* External reference */
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [Zero] = One
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [One] = CPNU (\_SB.PL21, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x02] = CPNU (\_SB.PL21, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x03] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x04] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x05] = PPSZ /* External reference */
        }

        Method (CPL2, 0, NotSerialized)
        {
            \_SB.PC00.TCPU.NPCC [Zero] = 0x02
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [Zero] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [One] = 0x7D
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x02] = CPNU (\_SB.PL12, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x03] = (\_SB.PLW2 * 0x03E8)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x04] = ((\_SB.PLW2 * 0x03E8
                ) + 0x0FA0)
            DerefOf (\_SB.PC00.TCPU.NPCC [One]) [0x05] = PPSZ /* External reference */
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [Zero] = One
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [One] = CPNU (\_SB.PL22, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x02] = CPNU (\_SB.PL22, One)
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x03] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x04] = Zero
            DerefOf (\_SB.PC00.TCPU.NPCC [0x02]) [0x05] = PPSZ /* External reference */
        }

        Name (LSTM, Zero)
        Name (_PPC, Zero)  // _PPC: Performance Present Capabilities
        Method (SPPC, 1, Serialized)
        {
            If (CondRefOf (\_SB.CPPC))
            {
                \_SB.CPPC = Arg0
            }

            Switch (ToInteger (\TCNT))
            {
                Case (0x10)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                    Notify (\_SB.PR05, 0x80) // Status Change
                    Notify (\_SB.PR06, 0x80) // Status Change
                    Notify (\_SB.PR07, 0x80) // Status Change
                    Notify (\_SB.PR08, 0x80) // Status Change
                    Notify (\_SB.PR09, 0x80) // Status Change
                    Notify (\_SB.PR10, 0x80) // Status Change
                    Notify (\_SB.PR11, 0x80) // Status Change
                    Notify (\_SB.PR12, 0x80) // Status Change
                    Notify (\_SB.PR13, 0x80) // Status Change
                    Notify (\_SB.PR14, 0x80) // Status Change
                    Notify (\_SB.PR15, 0x80) // Status Change
                }
                Case (0x0E)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                    Notify (\_SB.PR05, 0x80) // Status Change
                    Notify (\_SB.PR06, 0x80) // Status Change
                    Notify (\_SB.PR07, 0x80) // Status Change
                    Notify (\_SB.PR08, 0x80) // Status Change
                    Notify (\_SB.PR09, 0x80) // Status Change
                    Notify (\_SB.PR10, 0x80) // Status Change
                    Notify (\_SB.PR11, 0x80) // Status Change
                    Notify (\_SB.PR12, 0x80) // Status Change
                    Notify (\_SB.PR13, 0x80) // Status Change
                }
                Case (0x0C)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                    Notify (\_SB.PR05, 0x80) // Status Change
                    Notify (\_SB.PR06, 0x80) // Status Change
                    Notify (\_SB.PR07, 0x80) // Status Change
                    Notify (\_SB.PR08, 0x80) // Status Change
                    Notify (\_SB.PR09, 0x80) // Status Change
                    Notify (\_SB.PR10, 0x80) // Status Change
                    Notify (\_SB.PR11, 0x80) // Status Change
                }
                Case (0x0A)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                    Notify (\_SB.PR05, 0x80) // Status Change
                    Notify (\_SB.PR06, 0x80) // Status Change
                    Notify (\_SB.PR07, 0x80) // Status Change
                    Notify (\_SB.PR08, 0x80) // Status Change
                    Notify (\_SB.PR09, 0x80) // Status Change
                }
                Case (0x08)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                    Notify (\_SB.PR05, 0x80) // Status Change
                    Notify (\_SB.PR06, 0x80) // Status Change
                    Notify (\_SB.PR07, 0x80) // Status Change
                }
                Case (0x07)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                    Notify (\_SB.PR05, 0x80) // Status Change
                    Notify (\_SB.PR06, 0x80) // Status Change
                }
                Case (0x06)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                    Notify (\_SB.PR05, 0x80) // Status Change
                }
                Case (0x05)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                    Notify (\_SB.PR04, 0x80) // Status Change
                }
                Case (0x04)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                    Notify (\_SB.PR03, 0x80) // Status Change
                }
                Case (0x03)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                    Notify (\_SB.PR02, 0x80) // Status Change
                }
                Case (0x02)
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                    Notify (\_SB.PR01, 0x80) // Status Change
                }
                Default
                {
                    Notify (\_SB.PR00, 0x80) // Status Change
                }

            }
        }

        Method (SPUR, 1, NotSerialized)
        {
            If ((Arg0 <= \TCNT))
            {
                If ((\_SB.PAGD._STA () == 0x0F))
                {
                    \_SB.PAGD._PUR [One] = Arg0
                    Notify (\_SB.PAGD, 0x80) // Status Change
                }
            }
        }

        Method (PCCC, 0, Serialized)
        {
            PCCX [Zero] = One
            Switch (ToInteger (CPNU (PTDP, Zero)))
            {
                Case (0x39)
                {
                    DerefOf (PCCX [One]) [Zero] = 0xA7F8
                    DerefOf (PCCX [One]) [One] = 0x00017318
                }
                Case (0x2F)
                {
                    DerefOf (PCCX [One]) [Zero] = 0x9858
                    DerefOf (PCCX [One]) [One] = 0x00014C08
                }
                Case (0x25)
                {
                    DerefOf (PCCX [One]) [Zero] = 0x7148
                    DerefOf (PCCX [One]) [One] = 0xD6D8
                }
                Case (0x19)
                {
                    DerefOf (PCCX [One]) [Zero] = 0x3E80
                    DerefOf (PCCX [One]) [One] = 0x7D00
                }
                Case (0x0F)
                {
                    DerefOf (PCCX [One]) [Zero] = 0x36B0
                    DerefOf (PCCX [One]) [One] = 0x7D00
                }
                Case (0x0B)
                {
                    DerefOf (PCCX [One]) [Zero] = 0x36B0
                    DerefOf (PCCX [One]) [One] = 0x61A8
                }
                Default
                {
                    DerefOf (PCCX [One]) [Zero] = 0xFF
                    DerefOf (PCCX [One]) [One] = 0xFF
                }

            }

            Return (PCCX) /* \_SB_.PC00.TCPU.PCCX */
        }

        Name (PCCX, Package (0x02)
        {
            0x80000000, 
            Package (0x02)
            {
                0x80000000, 
                0x80000000
            }
        })
        Name (KEFF, Package (0x1E)
        {
            Package (0x02)
            {
                0x01BC, 
                Zero
            }, 

            Package (0x02)
            {
                0x01CF, 
                0x27
            }, 

            Package (0x02)
            {
                0x01E1, 
                0x4B
            }, 

            Package (0x02)
            {
                0x01F3, 
                0x6C
            }, 

            Package (0x02)
            {
                0x0206, 
                0x8B
            }, 

            Package (0x02)
            {
                0x0218, 
                0xA8
            }, 

            Package (0x02)
            {
                0x022A, 
                0xC3
            }, 

            Package (0x02)
            {
                0x023D, 
                0xDD
            }, 

            Package (0x02)
            {
                0x024F, 
                0xF4
            }, 

            Package (0x02)
            {
                0x0261, 
                0x010B
            }, 

            Package (0x02)
            {
                0x0274, 
                0x011F
            }, 

            Package (0x02)
            {
                0x032C, 
                0x01BD
            }, 

            Package (0x02)
            {
                0x03D7, 
                0x0227
            }, 

            Package (0x02)
            {
                0x048B, 
                0x026D
            }, 

            Package (0x02)
            {
                0x053E, 
                0x02A1
            }, 

            Package (0x02)
            {
                0x05F7, 
                0x02C6
            }, 

            Package (0x02)
            {
                0x06A8, 
                0x02E6
            }, 

            Package (0x02)
            {
                0x075D, 
                0x02FF
            }, 

            Package (0x02)
            {
                0x0818, 
                0x0311
            }, 

            Package (0x02)
            {
                0x08CF, 
                0x0322
            }, 

            Package (0x02)
            {
                0x179C, 
                0x0381
            }, 

            Package (0x02)
            {
                0x2DDC, 
                0x039C
            }, 

            Package (0x02)
            {
                0x44A8, 
                0x039E
            }, 

            Package (0x02)
            {
                0x5C35, 
                0x0397
            }, 

            Package (0x02)
            {
                0x747D, 
                0x038D
            }, 

            Package (0x02)
            {
                0x8D7F, 
                0x0382
            }, 

            Package (0x02)
            {
                0xA768, 
                0x0376
            }, 

            Package (0x02)
            {
                0xC23B, 
                0x0369
            }, 

            Package (0x02)
            {
                0xDE26, 
                0x035A
            }, 

            Package (0x02)
            {
                0xFB7C, 
                0x034A
            }
        })
        Name (CEUP, Package (0x06)
        {
            0x80000000, 
            0x80000000, 
            0x80000000, 
            0x80000000, 
            0x80000000, 
            0x80000000
        })
        Method (_TMP, 0, Serialized)  // _TMP: Temperature
        {
            Return (\_SB.IETM.CTOK (PCTP))
        }

        Method (_DTI, 1, NotSerialized)  // _DTI: Device Temperature Indication
        {
            LSTM = Arg0
            Notify (\_SB.PC00.TCPU, 0x91) // Device-Specific
        }

        Method (_NTT, 0, NotSerialized)  // _NTT: Notification Temperature Threshold
        {
            Return (0x0ADE)
        }

        Name (PTYP, Zero)
        Method (_PSS, 0, NotSerialized)  // _PSS: Performance Supported States
        {
            If (CondRefOf (\_SB.PR00._PSS))
            {
                Return (\_SB.PR00._PSS ())
            }
            Else
            {
                Return (Package (0x02)
                {
                    Package (0x06)
                    {
                        Zero, 
                        Zero, 
                        Zero, 
                        Zero, 
                        Zero, 
                        Zero
                    }, 

                    Package (0x06)
                    {
                        Zero, 
                        Zero, 
                        Zero, 
                        Zero, 
                        Zero, 
                        Zero
                    }
                })
            }
        }

        Method (_TSS, 0, NotSerialized)  // _TSS: Throttling Supported States
        {
            If (CondRefOf (\_SB.PR00._TSS))
            {
                Return (\_SB.PR00._TSS ())
            }
            Else
            {
                Return (Package (0x01)
                {
                    Package (0x05)
                    {
                        One, 
                        Zero, 
                        Zero, 
                        Zero, 
                        Zero
                    }
                })
            }
        }

        Method (_TPC, 0, NotSerialized)  // _TPC: Throttling Present Capabilities
        {
            If (CondRefOf (\_SB.PR00._TPC))
            {
                Return (\_SB.PR00._TPC) /* External reference */
            }
            Else
            {
                Return (Zero)
            }
        }

        Method (_PTC, 0, NotSerialized)  // _PTC: Processor Throttling Control
        {
            If ((CondRefOf (\PF00) && (\PF00 != 0x80000000)))
            {
                If ((\PF00 & 0x04))
                {
                    Return (Package (0x02)
                    {
                        ResourceTemplate ()
                        {
                            Register (FFixedHW, 
                                0x00,               // Bit Width
                                0x00,               // Bit Offset
                                0x0000000000000000, // Address
                                ,)
                        }, 

                        ResourceTemplate ()
                        {
                            Register (FFixedHW, 
                                0x00,               // Bit Width
                                0x00,               // Bit Offset
                                0x0000000000000000, // Address
                                ,)
                        }
                    })
                }
                Else
                {
                    Return (Package (0x02)
                    {
                        ResourceTemplate ()
                        {
                            Register (SystemIO, 
                                0x05,               // Bit Width
                                0x00,               // Bit Offset
                                0x0000000000001810, // Address
                                ,)
                        }, 

                        ResourceTemplate ()
                        {
                            Register (SystemIO, 
                                0x05,               // Bit Width
                                0x00,               // Bit Offset
                                0x0000000000001810, // Address
                                ,)
                        }
                    })
                }
            }
            Else
            {
                Return (Package (0x02)
                {
                    ResourceTemplate ()
                    {
                        Register (FFixedHW, 
                            0x00,               // Bit Width
                            0x00,               // Bit Offset
                            0x0000000000000000, // Address
                            ,)
                    }, 

                    ResourceTemplate ()
                    {
                        Register (FFixedHW, 
                            0x00,               // Bit Width
                            0x00,               // Bit Offset
                            0x0000000000000000, // Address
                            ,)
                    }
                })
            }
        }

        Method (_TSD, 0, NotSerialized)  // _TSD: Throttling State Dependencies
        {
            If (CondRefOf (\_SB.PR00._TSD))
            {
                Return (\_SB.PR00._TSD ())
            }
            Else
            {
                Return (Package (0x01)
                {
                    Package (0x05)
                    {
                        0x05, 
                        Zero, 
                        Zero, 
                        0xFC, 
                        Zero
                    }
                })
            }
        }

        Method (_TDL, 0, NotSerialized)  // _TDL: T-State Depth Limit
        {
            If ((CondRefOf (\_SB.PR00._TSS) && CondRefOf (\_SB.CFGD)))
            {
                If ((\_SB.CFGD & 0x2000))
                {
                    Return ((SizeOf (\_SB.PR00.TSMF) - One))
                }
                Else
                {
                    Return ((SizeOf (\_SB.PR00.TSMC) - One))
                }
            }
            Else
            {
                Return (Zero)
            }
        }

        Method (_PDL, 0, NotSerialized)  // _PDL: P-state Depth Limit
        {
            If (CondRefOf (\_SB.PR00._PSS))
            {
                If ((\_SB.OSCP & 0x0400))
                {
                    Return ((SizeOf (\_SB.PR00.TPSS) - One))
                }
                Else
                {
                    Return ((SizeOf (\_SB.PR00.LPSS) - One))
                }
            }
            Else
            {
                Return (Zero)
            }
        }

        Name (TJMX, 0x6E)
        Method (_TSP, 0, Serialized)  // _TSP: Thermal Sampling Period
        {
            Return (Zero)
        }

        Method (_AC0, 0, Serialized)  // _ACx: Active Cooling, x=0-9
        {
            Local1 = \_SB.IETM.CTOK (TJMX)
            Local1 -= 0x0A
            If ((LSTM >= Local1))
            {
                Return ((Local1 - 0x14))
            }
            Else
            {
                Return (Local1)
            }
        }

        Method (_AC1, 0, Serialized)  // _ACx: Active Cooling, x=0-9
        {
            Local1 = \_SB.IETM.CTOK (TJMX)
            Local1 -= 0x1E
            If ((LSTM >= Local1))
            {
                Return ((Local1 - 0x14))
            }
            Else
            {
                Return (Local1)
            }
        }

        Method (_AC2, 0, Serialized)  // _ACx: Active Cooling, x=0-9
        {
            Local1 = \_SB.IETM.CTOK (TJMX)
            Local1 -= 0x28
            If ((LSTM >= Local1))
            {
                Return ((Local1 - 0x14))
            }
            Else
            {
                Return (Local1)
            }
        }

        Method (_AC3, 0, Serialized)  // _ACx: Active Cooling, x=0-9
        {
            Local1 = \_SB.IETM.CTOK (TJMX)
            Local1 -= 0x37
            If ((LSTM >= Local1))
            {
                Return ((Local1 - 0x14))
            }
            Else
            {
                Return (Local1)
            }
        }

        Method (_AC4, 0, Serialized)  // _ACx: Active Cooling, x=0-9
        {
            Local1 = \_SB.IETM.CTOK (TJMX)
            Local1 -= 0x46
            If ((LSTM >= Local1))
            {
                Return ((Local1 - 0x14))
            }
            Else
            {
                Return (Local1)
            }
        }

        Method (_PSV, 0, Serialized)  // _PSV: Passive Temperature
        {
            Return (\_SB.IETM.CTOK (TJMX))
        }

        Method (_CRT, 0, Serialized)  // _CRT: Critical Temperature
        {
            Return (\_SB.IETM.CTOK (TJMX))
        }

        Method (_CR3, 0, Serialized)  // _CR3: Warm/Standby Temperature
        {
            Return (\_SB.IETM.CTOK (TJMX))
        }

        Method (_HOT, 0, Serialized)  // _HOT: Hot Temperature
        {
            Return (\_SB.IETM.CTOK (TJMX))
        }

        Method (UVTH, 1, Serialized)
        {
        }
    }

    Scope (\_SB.IETM)
    {
        Name (CTSP, Package (0x01)
        {
            ToUUID ("e145970a-e4c1-4d73-900e-c9c5a69dd067") /* Unknown UUID */
        })
    }

    Scope (\_SB.PC00.TCPU)
    {
        Method (TDPL, 0, Serialized)
        {
            Name (AAAA, Zero)
            Name (BBBB, Zero)
            Name (CCCC, Zero)
            Local0 = CTNL /* \_SB_.PC00.TCPU.CTNL */
            If (((Local0 == One) || (Local0 == 0x02)))
            {
                Local0 = \_SB.CLVL /* External reference */
            }
            Else
            {
                Return (Package (0x01)
                {
                    Zero
                })
            }

            If ((CLCK == One))
            {
                Local0 = One
            }

            AAAA = CPNU (\_SB.PL10, One)
            BBBB = CPNU (\_SB.PL11, One)
            CCCC = CPNU (\_SB.PL12, One)
            Name (TMP1, Package (0x01)
            {
                Package (0x05)
                {
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000
                }
            })
            Name (TMP2, Package (0x02)
            {
                Package (0x05)
                {
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000
                }, 

                Package (0x05)
                {
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000
                }
            })
            Name (TMP3, Package (0x03)
            {
                Package (0x05)
                {
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000
                }, 

                Package (0x05)
                {
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000
                }, 

                Package (0x05)
                {
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000, 
                    0x80000000
                }
            })
            If ((Local0 == 0x03))
            {
                If ((AAAA > BBBB))
                {
                    If ((AAAA > CCCC))
                    {
                        If ((BBBB > CCCC))
                        {
                            Local3 = Zero
                            LEV0 = Zero
                            Local4 = One
                            LEV1 = One
                            Local5 = 0x02
                            LEV2 = 0x02
                        }
                        Else
                        {
                            Local3 = Zero
                            LEV0 = Zero
                            Local5 = One
                            LEV1 = 0x02
                            Local4 = 0x02
                            LEV2 = One
                        }
                    }
                    Else
                    {
                        Local5 = Zero
                        LEV0 = 0x02
                        Local3 = One
                        LEV1 = Zero
                        Local4 = 0x02
                        LEV2 = One
                    }
                }
                ElseIf ((BBBB > CCCC))
                {
                    If ((AAAA > CCCC))
                    {
                        Local4 = Zero
                        LEV0 = One
                        Local3 = One
                        LEV1 = Zero
                        Local5 = 0x02
                        LEV2 = 0x02
                    }
                    Else
                    {
                        Local4 = Zero
                        LEV0 = One
                        Local5 = One
                        LEV1 = 0x02
                        Local3 = 0x02
                        LEV2 = Zero
                    }
                }
                Else
                {
                    Local5 = Zero
                    LEV0 = 0x02
                    Local4 = One
                    LEV1 = One
                    Local3 = 0x02
                    LEV2 = Zero
                }

                Local1 = (\_SB.TAR0 + One)
                Local2 = (Local1 * 0x64)
                DerefOf (TMP3 [Local3]) [Zero] = AAAA /* \_SB_.PC00.TCPU.TDPL.AAAA */
                DerefOf (TMP3 [Local3]) [One] = Local2
                DerefOf (TMP3 [Local3]) [0x02] = \_SB.CTC0 /* External reference */
                DerefOf (TMP3 [Local3]) [0x03] = Local1
                DerefOf (TMP3 [Local3]) [0x04] = Zero
                Local1 = (\_SB.TAR1 + One)
                Local2 = (Local1 * 0x64)
                DerefOf (TMP3 [Local4]) [Zero] = BBBB /* \_SB_.PC00.TCPU.TDPL.BBBB */
                DerefOf (TMP3 [Local4]) [One] = Local2
                DerefOf (TMP3 [Local4]) [0x02] = \_SB.CTC1 /* External reference */
                DerefOf (TMP3 [Local4]) [0x03] = Local1
                DerefOf (TMP3 [Local4]) [0x04] = Zero
                Local1 = (\_SB.TAR2 + One)
                Local2 = (Local1 * 0x64)
                DerefOf (TMP3 [Local5]) [Zero] = CCCC /* \_SB_.PC00.TCPU.TDPL.CCCC */
                DerefOf (TMP3 [Local5]) [One] = Local2
                DerefOf (TMP3 [Local5]) [0x02] = \_SB.CTC2 /* External reference */
                DerefOf (TMP3 [Local5]) [0x03] = Local1
                DerefOf (TMP3 [Local5]) [0x04] = Zero
                Return (TMP3) /* \_SB_.PC00.TCPU.TDPL.TMP3 */
            }

            If ((Local0 == 0x02))
            {
                If ((AAAA > BBBB))
                {
                    Local3 = Zero
                    Local4 = One
                    LEV0 = Zero
                    LEV1 = One
                    LEV2 = Zero
                }
                Else
                {
                    Local4 = Zero
                    Local3 = One
                    LEV0 = One
                    LEV1 = Zero
                    LEV2 = Zero
                }

                Local1 = (\_SB.TAR0 + One)
                Local2 = (Local1 * 0x64)
                DerefOf (TMP2 [Local3]) [Zero] = AAAA /* \_SB_.PC00.TCPU.TDPL.AAAA */
                DerefOf (TMP2 [Local3]) [One] = Local2
                DerefOf (TMP2 [Local3]) [0x02] = \_SB.CTC0 /* External reference */
                DerefOf (TMP2 [Local3]) [0x03] = Local1
                DerefOf (TMP2 [Local3]) [0x04] = Zero
                Local1 = (\_SB.TAR1 + One)
                Local2 = (Local1 * 0x64)
                DerefOf (TMP2 [Local4]) [Zero] = BBBB /* \_SB_.PC00.TCPU.TDPL.BBBB */
                DerefOf (TMP2 [Local4]) [One] = Local2
                DerefOf (TMP2 [Local4]) [0x02] = \_SB.CTC1 /* External reference */
                DerefOf (TMP2 [Local4]) [0x03] = Local1
                DerefOf (TMP2 [Local4]) [0x04] = Zero
                Return (TMP2) /* \_SB_.PC00.TCPU.TDPL.TMP2 */
            }

            If ((Local0 == One))
            {
                Switch (ToInteger (\_SB.CBMI))
                {
                    Case (Zero)
                    {
                        Local1 = (\_SB.TAR0 + One)
                        Local2 = (Local1 * 0x64)
                        DerefOf (TMP1 [Zero]) [Zero] = AAAA /* \_SB_.PC00.TCPU.TDPL.AAAA */
                        DerefOf (TMP1 [Zero]) [One] = Local2
                        DerefOf (TMP1 [Zero]) [0x02] = \_SB.CTC0 /* External reference */
                        DerefOf (TMP1 [Zero]) [0x03] = Local1
                        DerefOf (TMP1 [Zero]) [0x04] = Zero
                        LEV0 = Zero
                        LEV1 = Zero
                        LEV2 = Zero
                    }
                    Case (One)
                    {
                        Local1 = (\_SB.TAR1 + One)
                        Local2 = (Local1 * 0x64)
                        DerefOf (TMP1 [Zero]) [Zero] = BBBB /* \_SB_.PC00.TCPU.TDPL.BBBB */
                        DerefOf (TMP1 [Zero]) [One] = Local2
                        DerefOf (TMP1 [Zero]) [0x02] = \_SB.CTC1 /* External reference */
                        DerefOf (TMP1 [Zero]) [0x03] = Local1
                        DerefOf (TMP1 [Zero]) [0x04] = Zero
                        LEV0 = One
                        LEV1 = One
                        LEV2 = One
                    }
                    Case (0x02)
                    {
                        Local1 = (\_SB.TAR2 + One)
                        Local2 = (Local1 * 0x64)
                        DerefOf (TMP1 [Zero]) [Zero] = CCCC /* \_SB_.PC00.TCPU.TDPL.CCCC */
                        DerefOf (TMP1 [Zero]) [One] = Local2
                        DerefOf (TMP1 [Zero]) [0x02] = \_SB.CTC2 /* External reference */
                        DerefOf (TMP1 [Zero]) [0x03] = Local1
                        DerefOf (TMP1 [Zero]) [0x04] = Zero
                        LEV0 = 0x02
                        LEV1 = 0x02
                        LEV2 = 0x02
                    }

                }

                Return (TMP1) /* \_SB_.PC00.TCPU.TDPL.TMP1 */
            }

            Return (Zero)
        }

        Name (MAXT, Zero)
        Method (TDPC, 0, NotSerialized)
        {
            Return (MAXT) /* \_SB_.PC00.TCPU.MAXT */
        }

        Name (LEV0, Zero)
        Name (LEV1, Zero)
        Name (LEV2, Zero)
        Method (STDP, 1, Serialized)
        {
            If ((Arg0 >= \_SB.CLVL))
            {
                Return (Zero)
            }

            Switch (ToInteger (Arg0))
            {
                Case (Zero)
                {
                    Local0 = LEV0 /* \_SB_.PC00.TCPU.LEV0 */
                }
                Case (One)
                {
                    Local0 = LEV1 /* \_SB_.PC00.TCPU.LEV1 */
                }
                Case (0x02)
                {
                    Local0 = LEV2 /* \_SB_.PC00.TCPU.LEV2 */
                }

            }

            Switch (ToInteger (Local0))
            {
                Case (Zero)
                {
                    CPL0 ()
                }
                Case (One)
                {
                    CPL1 ()
                }
                Case (0x02)
                {
                    CPL2 ()
                }

            }

            Notify (\_SB.PC00.TCPU, 0x83) // Device-Specific Change
        }
    }

    Scope (\_SB)
    {
        Device (TPCH)
        {
            Name (_HID, "INTC1045")  // _HID: Hardware ID
            Name (_UID, "TPCH")  // _UID: Unique ID
            Name (_STR, Unicode ("Intel PCH FIVR Participant"))  // _STR: Description String
            Name (PTYP, 0x05)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((\PCHE == One))
                {
                    Return (0x0F)
                }
                Else
                {
                    Return (Zero)
                }
            }

            Method (RFC0, 1, Serialized)
            {
                IPCS (0xA3, One, 0x08, Zero, Arg0, Zero, Zero)
                Return (Package (0x01)
                {
                    Zero
                })
            }

            Method (RFC1, 1, Serialized)
            {
                IPCS (0xA3, One, 0x08, One, Arg0, Zero, Zero)
                Return (Package (0x01)
                {
                    Zero
                })
            }

            Method (SEMI, 1, Serialized)
            {
                IPCS (0xA3, One, 0x08, 0x02, Arg0, Zero, Zero)
                Return (Package (0x01)
                {
                    Zero
                })
            }

            Method (PKGC, 1, Serialized)
            {
                Name (PPKG, Package (0x02)
                {
                    Zero, 
                    Zero
                })
                PPKG [Zero] = DerefOf (Arg0 [Zero])
                PPKG [One] = DerefOf (Arg0 [One])
                Return (PPKG) /* \_SB_.TPCH.PKGC.PPKG */
            }

            Method (GFC0, 0, Serialized)
            {
                Local0 = IPCS (0xA3, Zero, 0x08, Zero, Zero, Zero, Zero)
                Local1 = \_SB.TPCH.PKGC (Local0)
                Return (Local1)
            }

            Method (GFC1, 0, Serialized)
            {
                Local0 = IPCS (0xA3, Zero, 0x08, One, Zero, Zero, Zero)
                Local1 = \_SB.TPCH.PKGC (Local0)
                Return (Local1)
            }

            Method (GEMI, 0, Serialized)
            {
                Local0 = IPCS (0xA3, Zero, 0x08, 0x02, Zero, Zero, Zero)
                Local1 = \_SB.TPCH.PKGC (Local0)
                Return (Local1)
            }

            Method (GFFS, 0, Serialized)
            {
                Local0 = IPCS (0xA3, Zero, 0x08, 0x03, Zero, Zero, Zero)
                Local1 = \_SB.TPCH.PKGC (Local0)
                Return (Local1)
            }

            Method (GFCS, 0, Serialized)
            {
                Local0 = IPCS (0xA3, Zero, 0x08, 0x04, Zero, Zero, Zero)
                Local1 = \_SB.TPCH.PKGC (Local0)
                Return (Local1)
            }
        }
    }

    Scope (\_SB.IETM)
    {
        Name (TRT0, Package (0x02)
        {
            Package (0x08)
            {
                \_SB.PC00.TCPU, 
                \_SB.PC00.LPCB.H_EC.SEN2, 
                0x28, 
                0x64, 
                Zero, 
                Zero, 
                Zero, 
                Zero
            }, 

            Package (0x08)
            {
                \_SB.PC00.LPCB.H_EC.CHRG, 
                \_SB.PC00.LPCB.H_EC.SEN4, 
                0x14, 
                0xC8, 
                Zero, 
                Zero, 
                Zero, 
                Zero
            }
        })
        Method (_TRT, 0, NotSerialized)  // _TRT: Thermal Relationship Table
        {
            Return (TRT0) /* \_SB_.IETM.TRT0 */
        }
    }

    Scope (\_SB.IETM)
    {
        Name (PTTL, 0x14)
        Name (PSVT, Package (0x01)
        {
            0x02
        })
    }

    Scope (\_SB.IETM)
    {
        Name (DP2P, Package (0x01)
        {
            ToUUID ("9e04115a-ae87-4d1c-9500-0f3e340bfe75") /* Unknown UUID */
        })
        Name (DPSP, Package (0x01)
        {
            ToUUID ("42a441d6-ae6a-462b-a84b-4a8ce79027d3") /* Unknown UUID */
        })
        Name (DASP, Package (0x01)
        {
            ToUUID ("3a95c389-e4b8-4629-a526-c52c88626bae") /* Unknown UUID */
        })
        Name (DA2P, Package (0x01)
        {
            ToUUID ("0e56fab6-bdfc-4e8c-8246-40ecfd4d74ea") /* Unknown UUID */
        })
        Name (DCSP, Package (0x01)
        {
            ToUUID ("97c68ae7-15fa-499c-b8c9-5da81d606e0a") /* Unknown UUID */
        })
        Name (RFIP, Package (0x01)
        {
            ToUUID ("c4ce1849-243a-49f3-b8d5-f97002f38e6a") /* Unknown UUID */
        })
        Name (POBP, Package (0x01)
        {
            ToUUID ("f5a35014-c209-46a4-993a-eb56de7530a1") /* Unknown UUID */
        })
        Name (DAPP, Package (0x01)
        {
            ToUUID ("63be270f-1c11-48fd-a6f7-3af253ff3e2d") /* Unknown UUID */
        })
        Name (DVSP, Package (0x01)
        {
            ToUUID ("6ed722a7-9240-48a5-b479-31eef723d7cf") /* Unknown UUID */
        })
        Name (DPID, Package (0x01)
        {
            ToUUID ("42496e14-bc1b-46e8-a798-ca915464426f") /* Unknown UUID */
        })
    }

    Scope (\_SB.IETM)
    {
        Method (TEVT, 2, Serialized)
        {
            Switch (Arg0)
            {
                Case ("IETM")
                {
                    Notify (\_SB.IETM, Arg1)
                }
                Case ("TCPU")
                {
                    Notify (\_SB.PC00.TCPU, Arg1)
                }
                Case ("TPCH")
                {
                    Notify (\_SB.TPCH, Arg1)
                }

            }

            If (\ECON)
            {
                Switch (Arg0)
                {
                    Case ("CHRG")
                    {
                    }
                    Case ("SEN2")
                    {
                    }
                    Case ("SEN3")
                    {
                    }
                    Case ("SEN4")
                    {
                    }
                    Case ("SEN5")
                    {
                    }
                    Case ("TFN1")
                    {
                    }
                    Case ("TPWR")
                    {
                    }

                }
            }
        }
    }

    Scope (\_SB.IETM)
    {
        Method (GDDV, 0, Serialized)
        {
            Return (Package (0x01)
            {
                Buffer (0x05E1)
                {
                    /* 0000 */  0xE5, 0x1F, 0x94, 0x00, 0x00, 0x00, 0x00, 0x02,  // ........
                    /* 0008 */  0x00, 0x00, 0x00, 0x40, 0x67, 0x64, 0x64, 0x76,  // ...@gddv
                    /* 0010 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                    /* 0018 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                    /* 0020 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                    /* 0028 */  0x00, 0x00, 0x00, 0x00, 0x4F, 0x45, 0x4D, 0x20,  // ....OEM 
                    /* 0030 */  0x45, 0x78, 0x70, 0x6F, 0x72, 0x74, 0x65, 0x64,  // Exported
                    /* 0038 */  0x20, 0x44, 0x61, 0x74, 0x61, 0x56, 0x61, 0x75,  //  DataVau
                    /* 0040 */  0x6C, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // lt......
                    /* 0048 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                    /* 0050 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                    /* 0058 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                    /* 0060 */  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // ........
                    /* 0068 */  0x00, 0x00, 0x00, 0x00, 0x05, 0xC7, 0x15, 0x58,  // .......X
                    /* 0070 */  0xEF, 0x60, 0x45, 0xDC, 0x2D, 0x34, 0x7E, 0x53,  // .`E.-4~S
                    /* 0078 */  0x81, 0xAB, 0x08, 0xF2, 0xA7, 0xE6, 0x4D, 0x26,  // ......M&
                    /* 0080 */  0x08, 0xDB, 0x72, 0x0C, 0x74, 0xD2, 0x6C, 0x3B,  // ..r.t.l;
                    /* 0088 */  0xE4, 0x82, 0x70, 0x71, 0x4D, 0x05, 0x00, 0x00,  // ..pqM...
                    /* 0090 */  0x52, 0x45, 0x50, 0x4F, 0x5D, 0x00, 0x00, 0x00,  // REPO]...
                    /* 0098 */  0x01, 0x57, 0x5B, 0x00, 0x00, 0x00, 0x00, 0x00,  // .W[.....
                    /* 00A0 */  0x00, 0x00, 0x72, 0x87, 0xCD, 0xFF, 0x6D, 0x24,  // ..r...m$
                    /* 00A8 */  0x47, 0xDB, 0x3D, 0x24, 0x92, 0xB4, 0x16, 0x6F,  // G.=$...o
                    /* 00B0 */  0x45, 0xD8, 0xC3, 0xF5, 0x66, 0x14, 0x9F, 0x22,  // E...f.."
                    /* 00B8 */  0xD7, 0xF7, 0xDE, 0x67, 0x90, 0x9A, 0xA2, 0x0D,  // ...g....
                    /* 00C0 */  0x39, 0x25, 0xAD, 0xC3, 0x1A, 0xAD, 0x52, 0x0B,  // 9%....R.
                    /* 00C8 */  0x75, 0x38, 0xE1, 0xA4, 0x14, 0x44, 0x7B, 0x77,  // u8...D{w
                    /* 00D0 */  0x4D, 0xF1, 0x94, 0xD5, 0x2D, 0x46, 0x2D, 0x67,  // M...-F-g
                    /* 00D8 */  0x72, 0x4A, 0x13, 0x35, 0x25, 0x6E, 0x49, 0xA5,  // rJ.5%nI.
                    /* 00E0 */  0xFC, 0x41, 0x3F, 0x98, 0xD3, 0x2C, 0xBD, 0x8C,  // .A?..,..
                    /* 00E8 */  0xF4, 0xFA, 0x9A, 0x27, 0xD4, 0x65, 0x63, 0x75,  // ...'.ecu
                    /* 00F0 */  0x53, 0xBE, 0xF1, 0x9D, 0x4A, 0xC9, 0xDF, 0xF2,  // S...J...
                    /* 00F8 */  0x92, 0x16, 0x50, 0x36, 0x06, 0xC8, 0x65, 0xD8,  // ..P6..e.
                    /* 0100 */  0x93, 0xF6, 0x87, 0xCD, 0x69, 0xAF, 0xC7, 0x75,  // ....i..u
                    /* 0108 */  0x88, 0xE3, 0x2A, 0xA1, 0xE4, 0xC9, 0x9C, 0x0B,  // ..*.....
                    /* 0110 */  0x42, 0xAA, 0x2B, 0x22, 0x63, 0x10, 0x98, 0x8D,  // B.+"c...
                    /* 0118 */  0xD1, 0x41, 0x27, 0x17, 0xD6, 0xE5, 0x58, 0xC9,  // .A'...X.
                    /* 0120 */  0x8E, 0x3F, 0x84, 0x76, 0x7D, 0x82, 0xBC, 0x84,  // .?.v}...
                    /* 0128 */  0xBF, 0xFC, 0xBB, 0x40, 0xC7, 0xD6, 0xB6, 0x13,  // ...@....
                    /* 0130 */  0x08, 0x50, 0xF8, 0x80, 0xFA, 0x9E, 0x7F, 0x34,  // .P.....4
                    /* 0138 */  0x48, 0xDB, 0xBF, 0xB3, 0xC3, 0x9C, 0xF1, 0x2C,  // H......,
                    /* 0140 */  0x7D, 0xE3, 0x0C, 0xDB, 0x9F, 0x2E, 0xDE, 0x5A,  // }......Z
                    /* 0148 */  0x9B, 0xCA, 0xED, 0xF0, 0xD8, 0x3B, 0x8D, 0x35,  // .....;.5
                    /* 0150 */  0xCB, 0x87, 0x24, 0x3D, 0x57, 0x66, 0x9D, 0x8C,  // ..$=Wf..
                    /* 0158 */  0x57, 0x1B, 0x2B, 0xDC, 0x96, 0xAD, 0x8B, 0xBF,  // W.+.....
                    /* 0160 */  0x11, 0x51, 0x42, 0x14, 0xBB, 0x0E, 0xC2, 0x3C,  // .QB....<
                    /* 0168 */  0x21, 0x2A, 0xBE, 0xEB, 0xEA, 0x7E, 0xF5, 0xD8,  // !*...~..
                    /* 0170 */  0x4E, 0x3E, 0x16, 0x80, 0x1A, 0x43, 0x81, 0xCD,  // N>...C..
                    /* 0178 */  0xE3, 0xA6, 0xF0, 0xED, 0xAA, 0x87, 0x25, 0xCB,  // ......%.
                    /* 0180 */  0xC3, 0xDA, 0x9F, 0xB5, 0x2B, 0xE0, 0xAB, 0x3F,  // ....+..?
                    /* 0188 */  0xFF, 0xC6, 0x8E, 0xD4, 0x25, 0xF1, 0x15, 0x4A,  // ....%..J
                    /* 0190 */  0x5E, 0x11, 0xF7, 0x1E, 0x1C, 0xEA, 0xCB, 0x86,  // ^.......
                    /* 0198 */  0x10, 0xA4, 0x47, 0x92, 0x48, 0x18, 0x61, 0xC6,  // ..G.H.a.
                    /* 01A0 */  0xDB, 0xA5, 0xEA, 0x90, 0x08, 0xC2, 0xEC, 0xAC,  // ........
                    /* 01A8 */  0x69, 0x04, 0xD6, 0x75, 0x8F, 0x7B, 0xBE, 0x56,  // i..u.{.V
                    /* 01B0 */  0x40, 0x03, 0x4E, 0x65, 0xEB, 0xF2, 0x5D, 0x50,  // @.Ne..]P
                    /* 01B8 */  0xD8, 0xFB, 0x67, 0x93, 0x2F, 0xAE, 0xD1, 0x07,  // ..g./...
                    /* 01C0 */  0x08, 0x72, 0x1B, 0xA3, 0xA9, 0xB5, 0xFB, 0x15,  // .r......
                    /* 01C8 */  0x5F, 0x46, 0x89, 0x0C, 0x38, 0x6B, 0xD8, 0xF8,  // _F..8k..
                    /* 01D0 */  0xB4, 0x76, 0xEE, 0x52, 0x39, 0x11, 0x8B, 0xD8,  // .v.R9...
                    /* 01D8 */  0xF7, 0x61, 0x14, 0x8E, 0xA0, 0x33, 0x81, 0xAA,  // .a...3..
                    /* 01E0 */  0x3D, 0x2A, 0x1E, 0x5C, 0x77, 0x66, 0x7F, 0x07,  // =*.\wf..
                    /* 01E8 */  0xB2, 0xF7, 0x7F, 0x2B, 0xE7, 0x57, 0x1D, 0x63,  // ...+.W.c
                    /* 01F0 */  0x33, 0xDF, 0x9A, 0xE1, 0x71, 0x93, 0xB0, 0x45,  // 3...q..E
                    /* 01F8 */  0x08, 0x7A, 0xB7, 0x22, 0xC3, 0x20, 0x69, 0x72,  // .z.". ir
                    /* 0200 */  0x67, 0xAA, 0x30, 0x13, 0xEB, 0x8A, 0x02, 0xA1,  // g.0.....
                    /* 0208 */  0x4F, 0xC3, 0x09, 0xB8, 0xD3, 0xE0, 0x6E, 0xB7,  // O.....n.
                    /* 0210 */  0x22, 0xE6, 0xFA, 0xD7, 0xE5, 0xB0, 0x14, 0x9A,  // ".......
                    /* 0218 */  0xAB, 0x3E, 0xD7, 0xDD, 0x58, 0x3D, 0xA1, 0x42,  // .>..X=.B
                    /* 0220 */  0x87, 0x9F, 0xAE, 0xC4, 0xB6, 0x20, 0x28, 0x53,  // ..... (S
                    /* 0228 */  0xF8, 0x8E, 0x14, 0xB2, 0xC6, 0xF7, 0xCC, 0xBD,  // ........
                    /* 0230 */  0xBE, 0xA8, 0xB2, 0x0A, 0xEC, 0x46, 0x42, 0x94,  // .....FB.
                    /* 0238 */  0x54, 0x48, 0xD3, 0x2F, 0x29, 0xC7, 0x8B, 0x22,  // TH./).."
                    /* 0240 */  0xE9, 0x25, 0xDB, 0x7B, 0x56, 0x19, 0x48, 0x57,  // .%.{V.HW
                    /* 0248 */  0xB8, 0x8C, 0xB9, 0x97, 0xB3, 0xB9, 0x8B, 0x59,  // .......Y
                    /* 0250 */  0x9C, 0x8D, 0x99, 0x90, 0x4D, 0x44, 0xF9, 0x69,  // ....MD.i
                    /* 0258 */  0x3B, 0x9B, 0x03, 0xE0, 0xA5, 0x23, 0x0D, 0xC2,  // ;....#..
                    /* 0260 */  0x2D, 0xF5, 0xFE, 0x50, 0xDB, 0x19, 0x8F, 0x25,  // -..P...%
                    /* 0268 */  0x70, 0xF0, 0x11, 0x43, 0xDF, 0x3C, 0xE8, 0x5C,  // p..C.<.\
                    /* 0270 */  0xA6, 0xB6, 0xBD, 0x32, 0xDD, 0xDA, 0x53, 0xF2,  // ...2..S.
                    /* 0278 */  0xE9, 0xE5, 0xF9, 0xD8, 0x1C, 0xBA, 0xFB, 0x19,  // ........
                    /* 0280 */  0xF9, 0x57, 0x69, 0xB8, 0x51, 0xA3, 0x6F, 0xF4,  // .Wi.Q.o.
                    /* 0288 */  0x42, 0xBE, 0x1C, 0x30, 0x33, 0x8C, 0x64, 0x30,  // B..03.d0
                    /* 0290 */  0xFD, 0x84, 0x4C, 0x74, 0xB7, 0xAC, 0xB3, 0x55,  // ..Lt...U
                    /* 0298 */  0x6A, 0xE2, 0xAF, 0xCC, 0x59, 0x00, 0xBC, 0xCF,  // j...Y...
                    /* 02A0 */  0x72, 0xF2, 0x4E, 0x9D, 0xB6, 0x45, 0x08, 0xAC,  // r.N..E..
                    /* 02A8 */  0xDF, 0xEA, 0xF1, 0xA7, 0xC6, 0x4F, 0x5E, 0x01,  // .....O^.
                    /* 02B0 */  0xB3, 0x83, 0xB4, 0x31, 0x99, 0x17, 0x5F, 0x95,  // ...1.._.
                    /* 02B8 */  0x6C, 0xBC, 0xCD, 0x43, 0xB0, 0xEC, 0x32, 0x66,  // l..C..2f
                    /* 02C0 */  0x94, 0x15, 0xCD, 0x45, 0x4B, 0x13, 0xE9, 0x3F,  // ...EK..?
                    /* 02C8 */  0xCF, 0x3D, 0x65, 0x3B, 0x9F, 0xC5, 0x11, 0x56,  // .=e;...V
                    /* 02D0 */  0xA3, 0xEF, 0x47, 0x0F, 0x41, 0xF0, 0xE4, 0x7F,  // ..G.A...
                    /* 02D8 */  0x50, 0xBA, 0x6B, 0x48, 0x12, 0x03, 0xDB, 0x87,  // P.kH....
                    /* 02E0 */  0x0D, 0x2D, 0x6A, 0x65, 0x7D, 0x03, 0x8C, 0x15,  // .-je}...
                    /* 02E8 */  0x8A, 0x31, 0xB8, 0xCD, 0x6A, 0x9B, 0x40, 0x64,  // .1..j.@d
                    /* 02F0 */  0x7E, 0x72, 0xF0, 0x6A, 0x32, 0xC1, 0xBB, 0xD7,  // ~r.j2...
                    /* 02F8 */  0x70, 0xBA, 0xE1, 0xDB, 0x3F, 0x0E, 0x34, 0x84,  // p...?.4.
                    /* 0300 */  0x8A, 0xDC, 0xFD, 0xFE, 0x5B, 0xA6, 0xD2, 0x25,  // ....[..%
                    /* 0308 */  0x54, 0x82, 0x58, 0xC6, 0x2E, 0x3A, 0x22, 0x3B,  // T.X..:";
                    /* 0310 */  0xC1, 0xD1, 0x2C, 0x63, 0x68, 0x45, 0xD5, 0x13,  // ..,chE..
                    /* 0318 */  0xD2, 0x29, 0xB3, 0xD3, 0x35, 0x40, 0x0C, 0xBC,  // .)..5@..
                    /* 0320 */  0xCF, 0x32, 0x97, 0x83, 0xED, 0x73, 0x7F, 0x20,  // .2...s. 
                    /* 0328 */  0x04, 0x51, 0x16, 0xA4, 0x5B, 0xBB, 0xB2, 0x12,  // .Q..[...
                    /* 0330 */  0x37, 0xFC, 0xEA, 0x05, 0xD9, 0xF7, 0xE3, 0x4F,  // 7......O
                    /* 0338 */  0x11, 0xCA, 0x2C, 0xEC, 0x34, 0xC0, 0x1C, 0x91,  // ..,.4...
                    /* 0340 */  0xB5, 0x4D, 0xD4, 0x5E, 0x1D, 0xE3, 0x20, 0x18,  // .M.^.. .
                    /* 0348 */  0x29, 0x8B, 0x34, 0x83, 0x62, 0x42, 0x7D, 0xC0,  // ).4.bB}.
                    /* 0350 */  0xF0, 0xDD, 0x62, 0x89, 0xE1, 0xF3, 0x26, 0x8E,  // ..b...&.
                    /* 0358 */  0x2D, 0xD3, 0x5F, 0xEE, 0x9E, 0xB8, 0x84, 0xFD,  // -._.....
                    /* 0360 */  0x0F, 0xCD, 0xF1, 0x47, 0xE9, 0xAD, 0x7D, 0xD0,  // ...G..}.
                    /* 0368 */  0xBE, 0x38, 0xC5, 0xB2, 0x34, 0x02, 0xE9, 0x67,  // .8..4..g
                    /* 0370 */  0x41, 0x4E, 0xDD, 0xA9, 0x4B, 0x22, 0x82, 0x9C,  // AN..K"..
                    /* 0378 */  0x7A, 0xD6, 0xCD, 0xF5, 0x11, 0xA8, 0xB8, 0xD9,  // z.......
                    /* 0380 */  0xB3, 0x1D, 0x53, 0x43, 0xC3, 0x7B, 0x24, 0x02,  // ..SC.{$.
                    /* 0388 */  0xFE, 0xD3, 0xEF, 0x84, 0x7D, 0x5C, 0x63, 0xEE,  // ....}\c.
                    /* 0390 */  0xF6, 0x24, 0xEB, 0xA4, 0x74, 0xBE, 0x55, 0x4D,  // .$..t.UM
                    /* 0398 */  0x5A, 0xD3, 0x04, 0xC1, 0xEE, 0xE4, 0x47, 0x92,  // Z.....G.
                    /* 03A0 */  0x89, 0xB1, 0x44, 0x2A, 0x8E, 0x5C, 0x63, 0x94,  // ..D*.\c.
                    /* 03A8 */  0x4F, 0x89, 0xF3, 0x5E, 0x2E, 0x8C, 0xEF, 0xC5,  // O..^....
                    /* 03B0 */  0x5E, 0x52, 0x9E, 0x95, 0xBE, 0x7B, 0x04, 0x69,  // ^R...{.i
                    /* 03B8 */  0x94, 0x67, 0x4F, 0x26, 0xC4, 0x55, 0xD3, 0x65,  // .gO&.U.e
                    /* 03C0 */  0x60, 0xE4, 0xC7, 0xBF, 0x1E, 0x74, 0xA2, 0xA7,  // `....t..
                    /* 03C8 */  0x99, 0x94, 0xCB, 0x46, 0x0F, 0x7A, 0x7E, 0x8F,  // ...F.z~.
                    /* 03D0 */  0xD1, 0xD1, 0x6D, 0x6F, 0xD2, 0x6B, 0x69, 0x58,  // ..mo.kiX
                    /* 03D8 */  0xA9, 0xA0, 0xB5, 0x5D, 0x94, 0xDA, 0xCC, 0xC7,  // ...]....
                    /* 03E0 */  0x8D, 0x5A, 0xBE, 0xCB, 0xD3, 0xCA, 0x37, 0x25,  // .Z....7%
                    /* 03E8 */  0x1F, 0x09, 0x57, 0x56, 0xA9, 0x57, 0xF4, 0x8E,  // ..WV.W..
                    /* 03F0 */  0x7A, 0x06, 0xD7, 0xE3, 0xC1, 0xBC, 0x93, 0x76,  // z......v
                    /* 03F8 */  0xF8, 0x27, 0x19, 0x89, 0x6C, 0x87, 0x2D, 0xBC,  // .'..l.-.
                    /* 0400 */  0x7C, 0x55, 0x03, 0xA4, 0x13, 0xC8, 0x0D, 0xD9,  // |U......
                    /* 0408 */  0x3F, 0x97, 0xA4, 0x29, 0x91, 0x35, 0xFA, 0x4E,  // ?..).5.N
                    /* 0410 */  0x23, 0xBD, 0x57, 0xC3, 0x13, 0xA2, 0x9F, 0xB1,  // #.W.....
                    /* 0418 */  0x4F, 0xB5, 0xBA, 0xD2, 0xA1, 0x4C, 0x98, 0x25,  // O....L.%
                    /* 0420 */  0x18, 0xC4, 0xA8, 0xFD, 0xDA, 0x53, 0x94, 0x61,  // .....S.a
                    /* 0428 */  0x53, 0x80, 0x97, 0x2D, 0x34, 0xBF, 0xCF, 0xA8,  // S..-4...
                    /* 0430 */  0x0C, 0x11, 0x6F, 0x22, 0x98, 0x0B, 0x73, 0xB0,  // ..o"..s.
                    /* 0438 */  0x15, 0x02, 0x40, 0x3C, 0xEB, 0xE4, 0x42, 0x64,  // ..@<..Bd
                    /* 0440 */  0x58, 0xD2, 0x6B, 0xCE, 0xEB, 0x02, 0xEA, 0x4E,  // X.k....N
                    /* 0448 */  0x07, 0x28, 0xA7, 0xF7, 0x15, 0x65, 0x57, 0x83,  // .(...eW.
                    /* 0450 */  0x59, 0xF9, 0xC4, 0x37, 0x1F, 0x32, 0x75, 0x2D,  // Y..7.2u-
                    /* 0458 */  0x14, 0x78, 0x11, 0xD1, 0x8E, 0xB8, 0x3A, 0xA6,  // .x....:.
                    /* 0460 */  0x4C, 0x7B, 0xB5, 0xA7, 0x8E, 0x88, 0x5A, 0x28,  // L{....Z(
                    /* 0468 */  0x26, 0xE7, 0x84, 0x33, 0x27, 0xA0, 0xC7, 0x44,  // &..3'..D
                    /* 0470 */  0xCC, 0x7A, 0x83, 0xD5, 0xA3, 0x86, 0x88, 0x90,  // .z......
                    /* 0478 */  0xAF, 0xF6, 0x2F, 0xE8, 0xFC, 0x94, 0xF7, 0xDF,  // ../.....
                    /* 0480 */  0x53, 0xFD, 0x22, 0xBB, 0xC8, 0xF6, 0xEC, 0x7C,  // S."....|
                    /* 0488 */  0x12, 0x9B, 0x23, 0x72, 0x25, 0x7E, 0xA8, 0x23,  // ..#r%~.#
                    /* 0490 */  0xD2, 0x1C, 0xA2, 0x5D, 0xAC, 0x46, 0x91, 0x04,  // ...].F..
                    /* 0498 */  0x19, 0xCF, 0x2F, 0xE2, 0x7B, 0xED, 0xCB, 0xBC,  // ../.{...
                    /* 04A0 */  0x0C, 0x4B, 0x47, 0xF7, 0x0D, 0xFA, 0x55, 0x75,  // .KG...Uu
                    /* 04A8 */  0xDD, 0x88, 0x7C, 0xAC, 0x7E, 0x85, 0x09, 0x14,  // ..|.~...
                    /* 04B0 */  0x8F, 0xDA, 0x37, 0x92, 0xC0, 0x36, 0x14, 0x84,  // ..7..6..
                    /* 04B8 */  0x72, 0x97, 0x23, 0xE5, 0x0D, 0x47, 0x9F, 0x0F,  // r.#..G..
                    /* 04C0 */  0xA7, 0x4D, 0x98, 0x6E, 0xA5, 0x9E, 0x50, 0xFC,  // .M.n..P.
                    /* 04C8 */  0xCC, 0xF5, 0xA4, 0x46, 0xCC, 0x6E, 0x09, 0xCA,  // ...F.n..
                    /* 04D0 */  0xCE, 0x72, 0x2B, 0xD6, 0x8D, 0xC3, 0xE1, 0x8F,  // .r+.....
                    /* 04D8 */  0x70, 0x78, 0xE5, 0x44, 0x58, 0x80, 0x67, 0x5A,  // px.DX.gZ
                    /* 04E0 */  0x61, 0x69, 0xFB, 0x71, 0x6A, 0x61, 0x8E, 0x15,  // ai.qja..
                    /* 04E8 */  0xF3, 0x4E, 0xB6, 0x86, 0x10, 0xEB, 0x86, 0x8C,  // .N......
                    /* 04F0 */  0xB2, 0x02, 0xA8, 0x0A, 0x93, 0xB2, 0x51, 0xD6,  // ......Q.
                    /* 04F8 */  0x8C, 0xAD, 0x91, 0xBE, 0x4D, 0x23, 0x2C, 0xB0,  // ....M#,.
                    /* 0500 */  0x0D, 0x59, 0xF9, 0x70, 0x6C, 0xA7, 0xE0, 0x52,  // .Y.pl..R
                    /* 0508 */  0x88, 0x1B, 0xC6, 0xD0, 0xD6, 0xD5, 0xAC, 0xF9,  // ........
                    /* 0510 */  0xB7, 0x13, 0xF9, 0x8C, 0x84, 0x3A, 0x79, 0xD9,  // .....:y.
                    /* 0518 */  0x5B, 0xBB, 0x8D, 0x75, 0x74, 0xA6, 0x9E, 0x31,  // [..ut..1
                    /* 0520 */  0x7E, 0x20, 0x44, 0x67, 0xC4, 0x39, 0xEC, 0x72,  // ~ Dg.9.r
                    /* 0528 */  0x46, 0xFC, 0x14, 0xAD, 0xE1, 0xE9, 0x18, 0xE9,  // F.......
                    /* 0530 */  0xE5, 0xF9, 0x7B, 0x7B, 0x15, 0xFF, 0x33, 0x3F,  // ..{{..3?
                    /* 0538 */  0xC1, 0x8A, 0xAD, 0xEE, 0x46, 0x3F, 0x27, 0x68,  // ....F?'h
                    /* 0540 */  0xD9, 0x79, 0x91, 0x26, 0xE9, 0xCF, 0x2D, 0x4F,  // .y.&..-O
                    /* 0548 */  0x38, 0xAC, 0xE5, 0x32, 0xB1, 0x79, 0x8C, 0xD2,  // 8..2.y..
                    /* 0550 */  0x33, 0x15, 0x18, 0x2C, 0xCD, 0x70, 0x58, 0xE1,  // 3..,.pX.
                    /* 0558 */  0x07, 0x32, 0xEE, 0xC3, 0x3F, 0x0E, 0xDE, 0xD8,  // .2..?...
                    /* 0560 */  0x6E, 0x77, 0x07, 0x1E, 0x56, 0x30, 0x47, 0x35,  // nw..V0G5
                    /* 0568 */  0xEA, 0xCF, 0xD7, 0x3E, 0xDD, 0xFA, 0x4D, 0x49,  // ...>..MI
                    /* 0570 */  0x8E, 0x52, 0x97, 0xE0, 0xBA, 0x29, 0xFF, 0xE0,  // .R...)..
                    /* 0578 */  0xF8, 0x68, 0xD3, 0x9A, 0x23, 0xBB, 0x96, 0x1F,  // .h..#...
                    /* 0580 */  0x10, 0x10, 0xFE, 0x9C, 0xEE, 0xA2, 0x0F, 0x58,  // .......X
                    /* 0588 */  0x53, 0x91, 0x5D, 0x4E, 0xAB, 0xDC, 0x1D, 0x64,  // S.]N...d
                    /* 0590 */  0xF1, 0x69, 0x31, 0x81, 0xF6, 0xDD, 0xB1, 0x13,  // .i1.....
                    /* 0598 */  0x5E, 0x7E, 0xEE, 0x2D, 0x98, 0x08, 0xE5, 0xF3,  // ^~.-....
                    /* 05A0 */  0xDC, 0xD0, 0xA8, 0x73, 0xC9, 0x4C, 0xC0, 0x11,  // ...s.L..
                    /* 05A8 */  0x41, 0x1F, 0x91, 0x0F, 0xD1, 0x70, 0x3C, 0x25,  // A....p<%
                    /* 05B0 */  0xE1, 0x65, 0x97, 0xFF, 0x61, 0x9B, 0xED, 0x0D,  // .e..a...
                    /* 05B8 */  0x82, 0x44, 0x6D, 0xAD, 0x05, 0xF3, 0x29, 0x09,  // .Dm...).
                    /* 05C0 */  0x07, 0x99, 0x7C, 0x99, 0x4D, 0xE0, 0xDB, 0x96,  // ..|.M...
                    /* 05C8 */  0x49, 0x50, 0x5F, 0xF7, 0xDD, 0x23, 0xD0, 0xC7,  // IP_..#..
                    /* 05D0 */  0xE0, 0x7B, 0xEB, 0x92, 0x0F, 0x08, 0x21, 0x85,  // .{....!.
                    /* 05D8 */  0xB0, 0x17, 0x8F, 0x39, 0xCC, 0xFB, 0x1B, 0x51,  // ...9...Q
                    /* 05E0 */  0x36                                             // 6
                }
            })
        }

        Method (IMOK, 1, NotSerialized)
        {
            Return (Arg0)
        }
    }
}

